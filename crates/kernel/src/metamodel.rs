//! Immutable, validated descriptors. No language class hierarchy is built in.
use crate::{MetaclassId, MetamodelId, PropertyId};
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
    /// Name unique within this metamodel descriptor namespace.
    pub name: String,
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
/// Redefinitions, subsetting and opposites are not interpreted by this milestone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyDescriptor {
    /// Stable property identity, including when inherited through several parents.
    pub id: PropertyId,
    /// Descriptive name. Ambiguous effective names require future explicit redefinition metadata.
    pub name: String,
    /// Declaring class; determines the owning metamodel release.
    pub owner: MetaclassId,
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
}

/// Validated immutable registry, with deterministic effective-property queries.
#[derive(Clone, Debug)]
pub struct MetamodelRegistry {
    models: BTreeMap<MetamodelId, MetamodelDescriptor>,
    classes: BTreeMap<MetaclassId, MetaclassDescriptor>,
    properties: BTreeMap<PropertyId, PropertyDescriptor>,
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
        let mut registry = Self {
            models: BTreeMap::new(),
            classes: BTreeMap::new(),
            properties: BTreeMap::new(),
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
            if let Some(first) = names.insert((class.metamodel, class.name.clone()), id) {
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
        for property in properties {
            let id = property.id;
            registry.class(property.owner)?;
            if let ValueKind::Reference(target) = property.value_kind {
                registry.class(target)?;
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
        for (&id, ancestors) in &registry.ancestors {
            let mut names = BTreeMap::new();
            let mut effective = BTreeSet::new();
            for property in registry
                .properties
                .values()
                .filter(|p| ancestors.contains(&p.owner))
            {
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
        self.metamodel(self.class(self.property(id)?.owner)?.metamodel)
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
        Ok(self.properties.values().filter(move |p| p.owner == class))
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
