//! Immutable, validated descriptors. No language class hierarchy is built in.
use crate::{
    AssociationId, EnumerationId, EnumerationLiteralId, MetaclassId, MetamodelId, PropertyId,
};
use std::collections::{BTreeMap, BTreeSet};

/// An explicit release version, not a language-version assumption in storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    /// Major release number.
    pub major: u32,
    /// Minor release number, without implicit compatibility assumptions.
    pub minor: u32,
    /// Patch release number.
    pub patch: u32,
}

/// The exact metamodel release owning a group of class descriptors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetamodelDescriptor {
    /// Identity of this exact descriptor source/release.
    pub id: MetamodelId,
    /// Human-readable label, not an identifier used by semantic queries.
    pub name: String,
    /// Explicit release version.
    pub version: Version,
    /// Publication/artifact URI. Text is provenance, not semantic identity.
    pub uri: String,
}

/// A metaclass declaration; inheritance is data rather than Rust inheritance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaclassDescriptor {
    /// Stable descriptor identity.
    pub id: MetaclassId,
    /// Name unique within its package in this metamodel.
    pub name: String,
    /// Descriptive package path, not descriptor identity.
    pub package: Vec<String>,
    /// The release that declares this metaclass.
    pub metamodel: MetamodelId,
    /// Direct parents, including parents from other registered metamodels.
    pub direct_supertypes: BTreeSet<MetaclassId>,
    /// Abstract classes participate in queries but cannot be instantiated directly.
    pub is_abstract: bool,
}

/// Supported scalar storage domains. This is not the full KerML value system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueKind {
    /// Boolean primitive.
    Boolean,
    /// Signed 64-bit primitive, not yet the normative unbounded Integer domain.
    Integer,
    /// Unicode text value.
    String,
    /// A literal belonging to the identified enumeration domain.
    Enumeration(EnumerationId),
    /// References must target an instance of this class or a subclass.
    Reference(MetaclassId),
}

/// Number of values allowed in a slot; `None` means no upper bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Multiplicity {
    /// Minimum number of values in a computed/present slot, or a required declared slot.
    pub lower: usize,
    /// Inclusive maximum; `None` is unbounded.
    pub upper: Option<usize>,
}
impl Multiplicity {
    /// Zero or one value, represented by absence or a scalar.
    pub const OPTIONAL: Self = Self {
        lower: 0,
        upper: Some(1),
    };
    /// Exactly one scalar value.
    pub const ONE: Self = Self {
        lower: 1,
        upper: Some(1),
    };
    /// Zero or more values, represented by a collection when present.
    pub const MANY: Self = Self {
        lower: 0,
        upper: None,
    };
    pub(crate) fn accepts(self, count: usize) -> bool {
        count >= self.lower && self.upper.is_none_or(|upper| count <= upper)
    }
    pub(crate) fn scalar(self) -> bool {
        self.upper.is_some_and(|upper| upper <= 1)
    }
}

/// Structural contract for a property. Ownership determines its metamodel.
///
/// `composite` designates containment references (single owning slot, acyclic).
/// Association-owned ends are metadata, never manufactured class slots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDescriptor {
    /// Stable property identity, including when inherited through several parents.
    pub id: PropertyId,
    /// Descriptive name. Redefinition is resolved by identity, never by this name.
    pub name: String,
    /// Actual declaring class or association.
    pub owner: PropertyOwner,
    /// Primitive domain or allowed reference target metaclass.
    pub value_kind: ValueKind,
    /// Cardinality of values, independently of collection ordering.
    pub multiplicity: Multiplicity,
    /// Collection order is semantically significant (ignored for scalar bounds).
    pub ordered: bool,
    /// Duplicate values are forbidden.
    pub unique: bool,
    /// Values may only be supplied through semantic derivation.
    pub derived: bool,
    /// References designate containment; primitives cannot be composite.
    pub composite: bool,
    /// Direct property identities replaced in an inheriting context.
    pub redefines: BTreeSet<PropertyId>,
    /// Direct subset relationships; these do not themselves compute values.
    pub subsets: BTreeSet<PropertyId>,
    /// A derived union requires a later, explicit evaluator.
    pub derived_union: bool,
    /// Association identity, independent of end names or order.
    pub association: Option<AssociationId>,
    /// Other member ends of the association, including association-owned ends.
    pub opposite_ends: BTreeSet<PropertyId>,
}

/// Ownership in the metamodel, not containment in a model instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropertyOwner {
    Class(MetaclassId),
    Association(AssociationId),
}

/// Binary association metadata. No canonical link store is implied by registration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationDescriptor {
    pub id: AssociationId,
    pub name: String,
    pub package: Vec<String>,
    pub metamodel: MetamodelId,
    /// Source order is preserved; it does not define an authoritative storage end.
    pub member_ends: Vec<PropertyId>,
    pub navigable_owned_ends: BTreeSet<PropertyId>,
}

/// Named finite domain. Literal identities, not their display names, are values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumerationDescriptor {
    pub id: EnumerationId,
    pub name: String,
    pub package: Vec<String>,
    pub metamodel: MetamodelId,
    pub literals: BTreeMap<EnumerationLiteralId, String>,
}

/// Complete descriptor registration input, including non-class dependencies.
#[derive(Clone, Debug, Default)]
pub struct DescriptorSet {
    pub models: Vec<MetamodelDescriptor>,
    pub classes: Vec<MetaclassDescriptor>,
    pub properties: Vec<PropertyDescriptor>,
    pub associations: Vec<AssociationDescriptor>,
    pub enumerations: Vec<EnumerationDescriptor>,
}

/// Invalid descriptor input. Construction never publishes a partial registry.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MetamodelError {
    #[error("duplicate metamodel {0}")]
    DuplicateMetamodel(MetamodelId),
    #[error("duplicate metaclass {0}")]
    DuplicateClass(MetaclassId),
    #[error("duplicate property {0}")]
    DuplicateProperty(PropertyId),
    #[error("unknown metamodel {0}")]
    UnknownMetamodel(MetamodelId),
    #[error("unknown metaclass {0}")]
    UnknownClass(MetaclassId),
    #[error("unknown property {0}")]
    UnknownProperty(PropertyId),
    #[error("classes {first} and {second} have the same name in one metamodel")]
    ClassNameConflict {
        first: MetaclassId,
        second: MetaclassId,
    },
    #[error("inheritance cycle involving {0:?}")]
    InheritanceCycle(Vec<MetaclassId>),
    #[error("properties {first} and {second} are ambiguous on class {class}")]
    PropertyConflict {
        class: MetaclassId,
        first: PropertyId,
        second: PropertyId,
    },
    #[error("invalid multiplicity for {0}")]
    InvalidMultiplicity(PropertyId),
    #[error("primitive property {0} cannot be composite")]
    PrimitiveContainment(PropertyId),
    #[error("unknown association {0}")]
    UnknownAssociation(AssociationId),
    #[error("unknown enumeration {0}")]
    UnknownEnumeration(EnumerationId),
    #[error("invalid or duplicate association {0}")]
    InvalidAssociation(AssociationId),
    #[error("invalid or duplicate enumeration {0}")]
    InvalidEnumeration(EnumerationId),
    #[error("invalid structural metadata on property {0}")]
    InvalidPropertyMetadata(PropertyId),
    #[error("invalid redefinition {property} -> {base}")]
    InvalidRedefinition {
        property: PropertyId,
        base: PropertyId,
    },
    #[error("cyclic property metadata involving {0:?}")]
    PropertyCycle(Vec<PropertyId>),
}

/// Validated immutable registry, with deterministic effective-property queries.
#[derive(Clone, Debug)]
pub struct MetamodelRegistry {
    models: BTreeMap<MetamodelId, MetamodelDescriptor>,
    classes: BTreeMap<MetaclassId, MetaclassDescriptor>,
    properties: BTreeMap<PropertyId, PropertyDescriptor>,
    associations: BTreeMap<AssociationId, AssociationDescriptor>,
    enumerations: BTreeMap<EnumerationId, EnumerationDescriptor>,
    redefined: BTreeMap<PropertyId, BTreeSet<PropertyId>>,
    ancestors: BTreeMap<MetaclassId, BTreeSet<MetaclassId>>,
    effective: BTreeMap<MetaclassId, BTreeSet<PropertyId>>,
}
impl MetamodelRegistry {
    /// Validate all descriptors together, including forward cross-model references.
    pub fn new(
        models: impl IntoIterator<Item = MetamodelDescriptor>,
        classes: impl IntoIterator<Item = MetaclassDescriptor>,
        properties: impl IntoIterator<Item = PropertyDescriptor>,
    ) -> Result<Self, MetamodelError> {
        Self::from_descriptors(DescriptorSet {
            models: models.into_iter().collect(),
            classes: classes.into_iter().collect(),
            properties: properties.into_iter().collect(),
            ..DescriptorSet::default()
        })
    }
    /// Register a dependency-closed descriptor set atomically.
    pub fn from_descriptors(input: DescriptorSet) -> Result<Self, MetamodelError> {
        let DescriptorSet {
            models,
            classes,
            properties,
            associations,
            enumerations,
        } = input;
        let mut registry = Self {
            models: BTreeMap::new(),
            classes: BTreeMap::new(),
            properties: BTreeMap::new(),
            associations: BTreeMap::new(),
            enumerations: BTreeMap::new(),
            redefined: BTreeMap::new(),
            ancestors: BTreeMap::new(),
            effective: BTreeMap::new(),
        };
        for model in models {
            let id = model.id;
            if registry.models.insert(id, model).is_some() {
                return Err(MetamodelError::DuplicateMetamodel(id));
            }
        }
        let mut names = BTreeMap::new();
        for class in classes {
            let id = class.id;
            if !registry.models.contains_key(&class.metamodel) {
                return Err(MetamodelError::UnknownMetamodel(class.metamodel));
            }
            if registry.classes.contains_key(&id) {
                return Err(MetamodelError::DuplicateClass(id));
            }
            if let Some(first) = names.insert(
                (class.metamodel, class.package.clone(), class.name.clone()),
                id,
            ) {
                return Err(MetamodelError::ClassNameConflict { first, second: id });
            }
            registry.classes.insert(id, class);
        }
        let mut remaining = BTreeMap::new();
        let mut children: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for class in registry.classes.values() {
            remaining.insert(class.id, class.direct_supertypes.len());
            for parent in &class.direct_supertypes {
                registry.class(*parent)?;
                children.entry(*parent).or_default().insert(class.id);
            }
        }
        let mut ready: BTreeSet<_> = remaining
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        while let Some(id) = ready.pop_first() {
            let mut ancestors = BTreeSet::from([id]);
            for parent in &registry.classes[&id].direct_supertypes {
                ancestors.extend(registry.ancestors[parent].iter().copied());
            }
            registry.ancestors.insert(id, ancestors);
            for child in children.get(&id).into_iter().flatten() {
                if let Some(count) = remaining.get_mut(child) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(*child);
                    }
                }
            }
        }
        if registry.ancestors.len() != registry.classes.len() {
            return Err(MetamodelError::InheritanceCycle(
                remaining
                    .into_iter()
                    .filter(|(_, n)| *n > 0)
                    .map(|(id, _)| id)
                    .collect(),
            ));
        }
        for association in associations {
            registry.metamodel(association.metamodel)?;
            let id = association.id;
            if association.member_ends.len() != 2
                || association.member_ends[0] == association.member_ends[1]
                || registry.associations.insert(id, association).is_some()
            {
                return Err(MetamodelError::InvalidAssociation(id));
            }
        }
        let mut literals = BTreeSet::new();
        for enumeration in enumerations {
            registry.metamodel(enumeration.metamodel)?;
            let id = enumeration.id;
            let mut names = BTreeSet::new();
            if enumeration
                .literals
                .iter()
                .any(|(id, name)| !literals.insert(*id) || !names.insert(name.clone()))
                || registry.enumerations.insert(id, enumeration).is_some()
            {
                return Err(MetamodelError::InvalidEnumeration(id));
            }
        }
        for property in properties {
            let id = property.id;
            match property.owner {
                PropertyOwner::Class(owner) => {
                    registry.class(owner)?;
                }
                PropertyOwner::Association(owner) => {
                    registry.association(owner)?;
                }
            }
            if let ValueKind::Reference(target) = property.value_kind {
                registry.class(target)?;
            }
            if let ValueKind::Enumeration(target) = property.value_kind {
                registry.enumeration(target)?;
            }
            if property
                .multiplicity
                .upper
                .is_some_and(|upper| upper < property.multiplicity.lower)
            {
                return Err(MetamodelError::InvalidMultiplicity(id));
            }
            if property.composite && !matches!(property.value_kind, ValueKind::Reference(_)) {
                return Err(MetamodelError::PrimitiveContainment(id));
            }
            if registry.properties.insert(id, property).is_some() {
                return Err(MetamodelError::DuplicateProperty(id));
            }
        }
        registry.validate_metadata()?;
        for (&id, ancestors) in &registry.ancestors {
            let mut names = BTreeMap::new();
            let mut effective = BTreeSet::new();
            let candidates: BTreeSet<_> = registry
                .properties
                .values()
                .filter(|p| matches!(p.owner, PropertyOwner::Class(owner) if ancestors.contains(&owner)))
                .map(|p| p.id).collect();
            let replaced: BTreeSet<_> = candidates
                .iter()
                .flat_map(|p| registry.redefined[p].iter().copied())
                .collect();
            // Two sibling definitions replacing the same ancestor need an explicit
            // joining redefinition even when their names differ.
            let mut replacements = BTreeMap::new();
            for property in candidates
                .difference(&replaced)
                .map(|p| &registry.properties[p])
            {
                for base in &registry.redefined[&property.id] {
                    if let Some(first) = replacements.insert(*base, property.id) {
                        return Err(MetamodelError::PropertyConflict {
                            class: id,
                            first,
                            second: property.id,
                        });
                    }
                }
                if let Some(first) = names.insert(&property.name, property.id) {
                    return Err(MetamodelError::PropertyConflict {
                        class: id,
                        first,
                        second: property.id,
                    });
                }
                effective.insert(property.id);
            }
            registry.effective.insert(id, effective);
        }
        Ok(registry)
    }
    fn validate_metadata(&mut self) -> Result<(), MetamodelError> {
        for association in self.associations.values() {
            for end in &association.member_ends {
                let p = self.property(*end)?;
                let expected = association
                    .member_ends
                    .iter()
                    .copied()
                    .filter(|other| other != end)
                    .collect();
                if p.association != Some(association.id)
                    || p.opposite_ends != expected
                    || !matches!(p.value_kind, ValueKind::Reference(_))
                {
                    return Err(MetamodelError::InvalidAssociation(association.id));
                }
                if let PropertyOwner::Class(owner) = p.owner {
                    let opposite =
                        self.property(*p.opposite_ends.first().expect("validated binary end"))?;
                    if let ValueKind::Reference(context) = opposite.value_kind
                        && !self.is_subtype(owner, context)?
                    {
                        return Err(MetamodelError::InvalidAssociation(association.id));
                    }
                }
            }
            for end in &association.navigable_owned_ends {
                if !association.member_ends.contains(end)
                    || self.property(*end)?.owner != PropertyOwner::Association(association.id)
                {
                    return Err(MetamodelError::InvalidAssociation(association.id));
                }
            }
        }
        let mut edges = BTreeMap::new();
        for p in self.properties.values() {
            if p.derived_union && !p.derived {
                return Err(MetamodelError::InvalidPropertyMetadata(p.id));
            }
            if let Some(association) = p.association {
                if !self.association(association)?.member_ends.contains(&p.id) {
                    return Err(MetamodelError::InvalidPropertyMetadata(p.id));
                }
            } else if !p.opposite_ends.is_empty()
                || matches!(p.owner, PropertyOwner::Association(_))
            {
                return Err(MetamodelError::InvalidPropertyMetadata(p.id));
            }
            if let PropertyOwner::Association(owner) = p.owner
                && p.association != Some(owner)
            {
                return Err(MetamodelError::InvalidPropertyMetadata(p.id));
            }
            for base in p.redefines.iter().chain(&p.subsets) {
                self.property(*base)?;
            }
            edges.insert(p.id, p.redefines.union(&p.subsets).copied().collect());
        }
        let cycle = crate::model::cyclic_nodes(&edges);
        if !cycle.is_empty() {
            return Err(MetamodelError::PropertyCycle(cycle));
        }
        for p in self.properties.values() {
            let mut replaced = BTreeSet::new();
            let mut pending: Vec<_> = p.redefines.iter().copied().collect();
            while let Some(base) = pending.pop() {
                if replaced.insert(base) {
                    pending.extend(&self.properties[&base].redefines);
                }
            }
            self.redefined.insert(p.id, replaced);
        }
        for p in self.properties.values() {
            for base in &p.redefines {
                let b = self.property(*base)?;
                let context = self.property_context(p)?;
                let base_context = self.property_context(b)?;
                if context == base_context
                    || !self.is_subtype(context, base_context)?
                    || !self.compatible_value(p.value_kind, b.value_kind)?
                    || p.multiplicity.lower < b.multiplicity.lower
                    || b.multiplicity
                        .upper
                        .is_some_and(|upper| p.multiplicity.upper.is_none_or(|n| n > upper))
                    || (b.unique && !p.unique)
                    || (b.ordered && !p.ordered && !p.multiplicity.scalar())
                    || (b.composite && !p.composite)
                {
                    return Err(MetamodelError::InvalidRedefinition {
                        property: p.id,
                        base: *base,
                    });
                }
            }
            for base in &p.subsets {
                let b = self.property(*base)?;
                if !self.is_subtype(self.property_context(p)?, self.property_context(b)?)?
                    || !self.compatible_value(p.value_kind, b.value_kind)?
                {
                    return Err(MetamodelError::InvalidPropertyMetadata(p.id));
                }
            }
        }
        Ok(())
    }

    fn compatible_value(&self, value: ValueKind, base: ValueKind) -> Result<bool, MetamodelError> {
        match (value, base) {
            (ValueKind::Reference(value), ValueKind::Reference(base)) => {
                self.is_subtype(value, base)
            }
            _ => Ok(value == base),
        }
    }

    /// Structural context for subsetting/redefinition. An association-owned end
    /// is viewed from the opposite end's type, without creating a class slot.
    pub fn property_context(&self, p: &PropertyDescriptor) -> Result<MetaclassId, MetamodelError> {
        match p.owner {
            PropertyOwner::Class(owner) => Ok(owner),
            PropertyOwner::Association(_) => {
                let opposite = p
                    .opposite_ends
                    .first()
                    .ok_or(MetamodelError::InvalidPropertyMetadata(p.id))?;
                match self.property(*opposite)?.value_kind {
                    ValueKind::Reference(target) => Ok(target),
                    _ => Err(MetamodelError::InvalidPropertyMetadata(p.id)),
                }
            }
        }
    }

    /// Exact association declaration, including source-ordered member ends.
    pub fn association(&self, id: AssociationId) -> Result<&AssociationDescriptor, MetamodelError> {
        self.associations
            .get(&id)
            .ok_or(MetamodelError::UnknownAssociation(id))
    }

    /// Structural navigability from class ownership or an explicit navigable
    /// owned-end declaration. Member-end order is not directionality.
    pub fn is_navigable(&self, property: PropertyId) -> Result<bool, MetamodelError> {
        let p = self.property(property)?;
        Ok(match p.owner {
            PropertyOwner::Class(_) => true,
            PropertyOwner::Association(id) => self
                .association(id)?
                .navigable_owned_ends
                .contains(&property),
        })
    }

    /// Whether one slot can be the sole canonical storage surface for this end.
    /// Derived slots are read-only projections. Authored association slots require
    /// a unique class end with an unordered 0..* association-owned, non-navigable
    /// opposite. No independently writable inverse, inverse bound, or inverse
    /// order then needs a link store. Also supports a unique ordered 0..* class
    /// end paired with a non-derived, unordered 0..1 class end. Its ordered slot
    /// stores each link once; the scalar inverse is a read-only index projection.
    pub fn supports_slot_storage(&self, property: PropertyId) -> Result<bool, MetamodelError> {
        let p = self.property(property)?;
        if !matches!(p.owner, PropertyOwner::Class(_)) {
            return Ok(false);
        }
        if p.derived || p.association.is_none() {
            return Ok(true);
        }
        if !p.unique {
            return Ok(false);
        }
        if self.scalar_inverse(property)?.is_some() {
            return Ok(true);
        }
        for opposite in &p.opposite_ends {
            let q = self.property(*opposite)?;
            if !matches!(q.owner, PropertyOwner::Association(_))
                || self.is_navigable(q.id)?
                || q.ordered
                || q.multiplicity != Multiplicity::MANY
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Scalar inverse of a supported ordered one-to-many association storage end.
    /// This structural policy does not imply containment or language ownership.
    pub fn scalar_inverse(
        &self,
        property: PropertyId,
    ) -> Result<Option<PropertyId>, MetamodelError> {
        let p = self.property(property)?;
        if p.derived
            || !p.unique
            || !p.ordered
            || p.multiplicity != Multiplicity::MANY
            || !matches!(p.owner, PropertyOwner::Class(_))
            || p.association.is_none()
        {
            return Ok(None);
        }
        let Some(opposite) = p.opposite_ends.first() else {
            return Ok(None);
        };
        let q = self.property(*opposite)?;
        Ok((!q.derived
            && q.unique
            && !q.ordered
            && q.multiplicity
                == Multiplicity {
                    lower: 0,
                    upper: Some(1),
                }
            && matches!(q.owner, PropertyOwner::Class(_)))
        .then_some(q.id))
    }

    /// Canonical ordered end backing a scalar inverse projection, if supported.
    pub fn inverse_storage(
        &self,
        property: PropertyId,
    ) -> Result<Option<PropertyId>, MetamodelError> {
        let p = self.property(property)?;
        for opposite in &p.opposite_ends {
            if self.scalar_inverse(*opposite)? == Some(property) {
                return Ok(Some(*opposite));
            }
        }
        Ok(None)
    }

    /// Exact enumeration domain and literal identities.
    pub fn enumeration(&self, id: EnumerationId) -> Result<&EnumerationDescriptor, MetamodelError> {
        self.enumerations
            .get(&id)
            .ok_or(MetamodelError::UnknownEnumeration(id))
    }

    /// Resolve an inherited property identity to its single effective replacement.
    /// A replaced descriptor is queryable here but is never a second legal slot.
    pub fn resolve_property(
        &self,
        class: MetaclassId,
        property: PropertyId,
    ) -> Result<Option<&PropertyDescriptor>, MetamodelError> {
        self.property(property)?;
        Ok(self
            .effective_properties(class)?
            .find(|p| p.id == property || self.redefined[&p.id].contains(&property)))
    }

    /// Resolve a display name only after inheritance and redefinition validation.
    pub fn property_named(
        &self,
        class: MetaclassId,
        name: &str,
    ) -> Result<Option<&PropertyDescriptor>, MetamodelError> {
        Ok(self.effective_properties(class)?.find(|p| p.name == name))
    }

    /// Look up the release owning descriptors.
    pub fn metamodel(&self, id: MetamodelId) -> Result<&MetamodelDescriptor, MetamodelError> {
        self.models
            .get(&id)
            .ok_or(MetamodelError::UnknownMetamodel(id))
    }
    /// Look up a metaclass by identity.
    pub fn class(&self, id: MetaclassId) -> Result<&MetaclassDescriptor, MetamodelError> {
        self.classes
            .get(&id)
            .ok_or(MetamodelError::UnknownClass(id))
    }
    /// Look up a property by identity.
    pub fn property(&self, id: PropertyId) -> Result<&PropertyDescriptor, MetamodelError> {
        self.properties
            .get(&id)
            .ok_or(MetamodelError::UnknownProperty(id))
    }
    /// Exact release of the metaclass declaring a property.
    pub fn property_metamodel(
        &self,
        id: PropertyId,
    ) -> Result<&MetamodelDescriptor, MetamodelError> {
        let model = match self.property(id)?.owner {
            PropertyOwner::Class(owner) => self.class(owner)?.metamodel,
            PropertyOwner::Association(owner) => self.association(owner)?.metamodel,
        };
        self.metamodel(model)
    }
    /// Reflexive subtype query; unknown IDs are errors, never a silent `false`.
    pub fn is_subtype(
        &self,
        class: MetaclassId,
        base: MetaclassId,
    ) -> Result<bool, MetamodelError> {
        self.class(class)?;
        self.class(base)?;
        Ok(self.ancestors[&class].contains(&base))
    }
    /// Effective properties in ascending property-ID order, deduplicating diamonds.
    pub fn effective_properties(
        &self,
        class: MetaclassId,
    ) -> Result<impl Iterator<Item = &PropertyDescriptor>, MetamodelError> {
        self.class(class)?;
        Ok(self.effective[&class].iter().map(|id| &self.properties[id]))
    }
    /// Properties declared directly on this class, in identity order.
    pub fn declared_properties(
        &self,
        class: MetaclassId,
    ) -> Result<impl Iterator<Item = &PropertyDescriptor>, MetamodelError> {
        self.class(class)?;
        Ok(self
            .properties
            .values()
            .filter(move |p| p.owner == PropertyOwner::Class(class)))
    }
    /// Whether an instance may carry this property (ignoring derived write policy).
    pub fn is_legal(
        &self,
        class: MetaclassId,
        property: PropertyId,
    ) -> Result<bool, MetamodelError> {
        self.property(property)?;
        self.class(class)?;
        Ok(self.effective[&class].contains(&property))
    }
    pub(crate) fn supertypes(&self, class: MetaclassId) -> impl Iterator<Item = MetaclassId> + '_ {
        self.ancestors.get(&class).into_iter().flatten().copied()
    }
}
