//! Immutable, validated descriptors. No language class hierarchy is built in.
use crate::{
    AssociationId, EnumerationId, EnumerationLiteralId, MetaclassId, MetamodelId,
    PrimitiveDomainId, PropertyId,
};
use std::collections::{BTreeMap, BTreeSet};
#[path = "conformance.rs"]
mod conformance;
use conformance::upper_contained;
pub use conformance::*;

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
    /// Exact arbitrary precision integer carrier.
    Integer,
    /// Exact finite decimal literal carrier (not binary64).
    Real,
    /// Exact descriptor-identified primitive domain.
    Primitive(PrimitiveDomainId),
    /// Unicode text value.
    String,
    /// A literal belonging to the identified enumeration domain.
    Enumeration(EnumerationId),
    /// References must target an instance of this class or a subclass.
    Reference(MetaclassId),
}

/// Generic scalar representation supported by a primitive domain descriptor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveRepresentation {
    Boolean,
    String,
    Integer,
    Real,
}

/// Primitive identity is independent of its name and storage representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimitiveDescriptor {
    pub id: PrimitiveDomainId,
    pub name: String,
    pub representation: PrimitiveRepresentation,
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
    /// Direct value-set inclusions. Reflexive and cyclic graphs are representable;
    /// registration is not UML conformance certification. These do not compute values.
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
    /// Metamodel association ancestry, independent of model-level specialization.
    pub direct_supertypes: BTreeSet<AssociationId>,
    pub is_abstract: bool,
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
    /// Exact source evidence, separate from descriptor identity and runtime inference.
    pub sources: BTreeMap<DescriptorId, DescriptorSource>,
    pub reviews: Vec<DiagnosticReview>,
    pub primitives: Vec<PrimitiveDescriptor>,
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
    #[error("unknown primitive domain {0}")]
    UnknownPrimitive(PrimitiveDomainId),
    #[error("duplicate primitive domain {0}")]
    DuplicatePrimitive(PrimitiveDomainId),
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
    #[error("association inheritance cycle involving {0:?}")]
    AssociationInheritanceCycle(Vec<AssociationId>),
    #[error("unsupported association redefinition {property} -> {base} in {association}")]
    UnsupportedAssociationRedefinition {
        property: PropertyId,
        base: PropertyId,
        association: AssociationId,
    },
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
    /// Redefinition must follow strict context ancestry; subsetting is independent.
    #[error("cyclic property redefinition involving {0:?}")]
    PropertyCycle(Vec<PropertyId>),
}

/// Finite contributor graph, not computed union values. Consumers must depend on
/// the exact descriptor graph, including absence of additional incoming edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DerivedUnionFrontier {
    pub contributors: BTreeSet<PropertyId>,
    pub inspected: BTreeSet<PropertyId>,
    pub unsupported_relations: BTreeSet<(PropertyId, PropertyId)>,
}

/// Validated immutable registry, with deterministic effective-property queries.
#[derive(Clone, Debug)]
pub struct MetamodelRegistry {
    sources: BTreeMap<DescriptorId, DescriptorSource>,
    reviews: Vec<DiagnosticReview>,
    primitives: BTreeMap<PrimitiveDomainId, PrimitiveDescriptor>,
    effective_errors: BTreeMap<MetaclassId, MetamodelError>,
    models: BTreeMap<MetamodelId, MetamodelDescriptor>,
    classes: BTreeMap<MetaclassId, MetaclassDescriptor>,
    properties: BTreeMap<PropertyId, PropertyDescriptor>,
    associations: BTreeMap<AssociationId, AssociationDescriptor>,
    enumerations: BTreeMap<EnumerationId, EnumerationDescriptor>,
    association_ancestors: BTreeMap<AssociationId, BTreeSet<AssociationId>>,
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
            sources,
            mut reviews,
            primitives,
            models,
            classes,
            properties,
            associations,
            enumerations,
        } = input;
        reviews.sort();
        let mut registry = Self {
            sources,
            reviews,
            primitives: BTreeMap::new(),
            effective_errors: BTreeMap::new(),
            models: BTreeMap::new(),
            classes: BTreeMap::new(),
            properties: BTreeMap::new(),
            associations: BTreeMap::new(),
            enumerations: BTreeMap::new(),
            association_ancestors: BTreeMap::new(),
            redefined: BTreeMap::new(),
            ancestors: BTreeMap::new(),
            effective: BTreeMap::new(),
        };
        for primitive in primitives {
            let id = primitive.id;
            if registry.primitives.insert(id, primitive).is_some() {
                return Err(MetamodelError::DuplicatePrimitive(id));
            }
        }
        for model in models {
            let id = model.id;
            if registry.models.insert(id, model).is_some() {
                return Err(MetamodelError::DuplicateMetamodel(id));
            }
        }
        for class in classes {
            let id = class.id;
            if !registry.models.contains_key(&class.metamodel) {
                return Err(MetamodelError::UnknownMetamodel(class.metamodel));
            }
            if registry.classes.contains_key(&id) {
                return Err(MetamodelError::DuplicateClass(id));
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
        let edges: BTreeMap<_, _> = registry
            .associations
            .iter()
            .map(|(&id, a)| (id, a.direct_supertypes.clone()))
            .collect();
        for parents in edges.values() {
            for parent in parents {
                registry.association(*parent)?;
            }
        }
        let cycle = crate::model::cyclic_nodes(&edges);
        if !cycle.is_empty() {
            return Err(MetamodelError::AssociationInheritanceCycle(cycle));
        }
        for &id in edges.keys() {
            let mut ancestors = BTreeSet::new();
            let mut pending = vec![id];
            while let Some(next) = pending.pop() {
                if ancestors.insert(next) {
                    pending.extend(&edges[&next]);
                }
            }
            registry.association_ancestors.insert(id, ancestors);
        }
        let mut literals = BTreeSet::new();
        for enumeration in enumerations {
            registry.metamodel(enumeration.metamodel)?;
            let id = enumeration.id;
            if enumeration.literals.keys().any(|id| !literals.insert(*id))
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
            if let ValueKind::Primitive(target) = property.value_kind {
                registry.primitive(target)?;
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
            // Distinct surviving properties can redefine the same ancestor. Their
            // exact identities remain usable; only an ambiguous alias query fails.
            for property in candidates
                .difference(&replaced)
                .map(|p| &registry.properties[p])
            {
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
            }
            for end in &association.navigable_owned_ends {
                if !association.member_ends.contains(end)
                    || self.property(*end)?.owner != PropertyOwner::Association(association.id)
                {
                    return Err(MetamodelError::InvalidAssociation(association.id));
                }
            }
        }
        for p in self.properties.values() {
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
        }
        // Raw edges never become slot replacements merely by being recorded.
        // Only safe class-to-class inheritance edges enter this operational graph.
        let mut class_edges = BTreeMap::new();
        for p in self.properties.values() {
            let mut eligible = BTreeSet::new();
            if let PropertyOwner::Class(owner) = p.owner {
                if p.derived_union && !p.derived {
                    for (&class, ancestors) in &self.ancestors {
                        if ancestors.contains(&owner) {
                            self.effective_errors
                                .insert(class, MetamodelError::InvalidPropertyMetadata(p.id));
                        }
                    }
                }
                for base in &p.redefines {
                    let b = &self.properties[base];
                    if matches!(b.owner, PropertyOwner::Class(_)) {
                        if self.redefinition_context_valid(p, b)
                            && self.redefinition_contract_valid(p, b)
                        {
                            eligible.insert(*base);
                        } else {
                            for (&class, ancestors) in &self.ancestors {
                                if ancestors.contains(&owner) {
                                    self.effective_errors.insert(
                                        class,
                                        MetamodelError::InvalidRedefinition {
                                            property: p.id,
                                            base: *base,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
            class_edges.insert(p.id, eligible);
        }
        for p in self.properties.values() {
            let mut replaced = BTreeSet::new();
            let mut pending: Vec<_> = class_edges[&p.id].iter().copied().collect();
            while let Some(base) = pending.pop() {
                if replaced.insert(base) {
                    pending.extend(&class_edges[&base]);
                }
            }
            self.redefined.insert(p.id, replaced);
        }
        Ok(())
    }

    fn redefinition_context_valid(&self, p: &PropertyDescriptor, b: &PropertyDescriptor) -> bool {
        match (p.owner, b.owner) {
            (PropertyOwner::Class(owner), PropertyOwner::Class(base)) => {
                owner != base && self.ancestors[&owner].contains(&base)
            }
            _ => match (p.association, b.association) {
                (Some(owner), Some(base)) => {
                    owner != base && self.association_ancestors[&owner].contains(&base)
                }
                _ => false,
            },
        }
    }

    fn redefinition_contract_valid(&self, p: &PropertyDescriptor, b: &PropertyDescriptor) -> bool {
        self.compatible_value(p.value_kind, b.value_kind)
            .expect("validated domains")
            && p.multiplicity.lower >= b.multiplicity.lower
            && upper_contained(p, b)
            && (!b.composite || p.composite)
    }

    pub fn primitive(&self, id: PrimitiveDomainId) -> Result<&PrimitiveDescriptor, MetamodelError> {
        self.primitives
            .get(&id)
            .ok_or(MetamodelError::UnknownPrimitive(id))
    }

    /// Scalar carrier for validation/projection. Descriptor domains remain exact
    /// for type compatibility; sharing a carrier does not make two domains equal.
    pub fn storage_kind(&self, kind: ValueKind) -> Result<ValueKind, MetamodelError> {
        Ok(match kind {
            ValueKind::Primitive(id) => match self.primitive(id)?.representation {
                PrimitiveRepresentation::Boolean => ValueKind::Boolean,
                PrimitiveRepresentation::String => ValueKind::String,
                PrimitiveRepresentation::Integer => ValueKind::Integer,
                PrimitiveRepresentation::Real => ValueKind::Real,
            },
            other => other,
        })
    }

    /// Deterministic complete source map, including exact artifact digests.
    pub fn sources(&self) -> impl Iterator<Item = (&DescriptorId, &DescriptorSource)> {
        self.sources.iter()
    }

    /// Exact source evidence; its presence does not assert conformance.
    pub fn source(&self, id: DescriptorId) -> Option<&DescriptorSource> {
        self.sources.get(&id)
    }

    /// Optional authoring validation, independent of structural construction.
    pub fn validate_conformance(&self) -> ConformanceReport {
        MetamodelValidator::validate(self)
    }

    /// Exact recorded targets, with no implied runtime replacement role.
    pub fn declared_redefinitions(
        &self,
        property: PropertyId,
    ) -> Result<&BTreeSet<PropertyId>, MetamodelError> {
        Ok(&self.property(property)?.redefines)
    }

    /// Transitive safe class replacements in this class context. Association-owned
    /// ends never remove class properties or become fabricated class slots.
    pub fn effective_redefinitions_for_class(
        &self,
        property: PropertyId,
        context: MetaclassId,
    ) -> Result<BTreeSet<PropertyId>, MetamodelError> {
        self.property(property)?;
        let _ = self.effective_properties(context)?;
        Ok(if self.effective[&context].contains(&property) {
            self.redefined[&property].clone()
        } else {
            BTreeSet::new()
        })
    }

    fn compatible_value(&self, value: ValueKind, base: ValueKind) -> Result<bool, MetamodelError> {
        match (value, base) {
            (ValueKind::Reference(value), ValueKind::Reference(base)) => {
                self.is_subtype(value, base)
            }
            _ => Ok(value == base),
        }
    }

    /// Reachable subset targets, each visited once, including the starting property
    /// only if reached through an edge. Safe for reflexive/cyclic metadata.
    ///
    /// This is metadata reachability, not derived-union evaluation or a UML
    /// conformance verdict. A future evaluator must deduplicate contributions by
    /// identity and solve cyclic inclusions by a monotone fixed point. Incremental
    /// users must depend on the registry identity and all inspected adjacency sets.
    pub fn subset_closure(
        &self,
        property: PropertyId,
    ) -> Result<BTreeSet<PropertyId>, MetamodelError> {
        let mut pending: Vec<_> = self.property(property)?.subsets.iter().copied().collect();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if visited.insert(id) {
                pending.extend(self.property(id)?.subsets.iter().copied());
            }
        }
        Ok(visited)
    }

    /// Properties transitively subsetting this property, visited once in the
    /// reverse graph. Includes this property only when reached through a cycle.
    ///
    /// Supplies a finite metadata frontier for future union evaluators; it does
    /// not compute values, choose ordered-union ordering, or claim completeness
    /// of a derivation. Each edge is visited at most once after adjacency construction.
    pub fn subset_contributors(
        &self,
        property: PropertyId,
    ) -> Result<BTreeSet<PropertyId>, MetamodelError> {
        self.property(property)?;
        let mut incoming: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for p in self.properties.values() {
            for target in &p.subsets {
                incoming.entry(*target).or_default().push(p.id);
            }
        }
        let mut pending = incoming.get(&property).cloned().unwrap_or_default();
        let mut visited = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if visited.insert(id) {
                pending.extend(incoming.get(&id).into_iter().flatten().copied());
            }
        }
        Ok(visited)
    }

    /// Termination-safe union frontier over subsets and applicable class redefinitions.
    /// Unsupported context/type edges remain explicit; they never establish a value.
    pub fn derived_union_frontier(
        &self,
        property: PropertyId,
        context: MetaclassId,
    ) -> Result<DerivedUnionFrontier, MetamodelError> {
        self.property(property)?;
        self.class(context)?;
        let mut incoming: BTreeMap<PropertyId, BTreeSet<PropertyId>> = BTreeMap::new();
        let mut unsupported = BTreeSet::new();
        for p in self.properties.values() {
            for &base in &p.subsets {
                incoming.entry(base).or_default().insert(p.id);
                let b = &self.properties[&base];
                if !self.is_subtype(self.property_context(p)?, self.property_context(b)?)?
                    || !self.compatible_value(p.value_kind, b.value_kind)?
                    || !upper_contained(p, b)
                {
                    unsupported.insert((p.id, base));
                }
            }
            if self.is_legal(context, p.id)? {
                for &base in &self.redefined[&p.id] {
                    incoming.entry(base).or_default().insert(p.id);
                }
            }
        }
        let mut inspected = BTreeSet::new();
        let mut contributors = BTreeSet::new();
        let mut pending = vec![property];
        let mut unsupported_relations = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if inspected.insert(id) {
                for &p in incoming.get(&id).into_iter().flatten() {
                    contributors.insert(p);
                    pending.push(p);
                    if unsupported.contains(&(p, id)) {
                        unsupported_relations.insert((p, id));
                    }
                }
            }
        }
        Ok(DerivedUnionFrontier {
            contributors,
            inspected,
            unsupported_relations,
        })
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

    /// Reflexive association ancestry, deterministic and independent of classes.
    pub fn association_ancestry(
        &self,
        id: AssociationId,
    ) -> Result<&BTreeSet<AssociationId>, MetamodelError> {
        self.association(id)?;
        Ok(&self.association_ancestors[&id])
    }

    /// All inherited member-end identities before operational redefinition filtering.
    pub fn inherited_association_ends(
        &self,
        id: AssociationId,
    ) -> Result<BTreeSet<PropertyId>, MetamodelError> {
        Ok(self
            .association_ancestry(id)?
            .iter()
            .flat_map(|a| self.associations[a].member_ends.iter().copied())
            .collect())
    }

    /// Interpret a particular recorded edge in an association hierarchy. A relation
    /// without the required hierarchy/contract is explicit unsupported semantics.
    pub fn interpret_association_redefinition(
        &self,
        property: PropertyId,
        base: PropertyId,
        context: AssociationId,
    ) -> Result<(), MetamodelError> {
        let p = self.property(property)?;
        let b = self.property(base)?;
        let ancestors = self.association_ancestry(context)?;
        if p.redefines.contains(&base)
            && self.is_subtype(self.property_context(p)?, self.property_context(b)?)?
            && self.redefinition_contract_valid(p, b)
            && p.association
                .zip(b.association)
                .is_some_and(|(owner, parent)| {
                    owner != parent
                        && ancestors.contains(&owner)
                        && self.association_ancestors[&owner].contains(&parent)
                })
        {
            Ok(())
        } else {
            Err(MetamodelError::UnsupportedAssociationRedefinition {
                property,
                base,
                association: context,
            })
        }
    }

    /// Inherited ends with only applicable association-hierarchy replacements.
    /// Diamonds deduplicate identities. Unrelated raw edges have no replacement role.
    pub fn effective_association_ends(
        &self,
        id: AssociationId,
    ) -> Result<BTreeSet<PropertyId>, MetamodelError> {
        let candidates = self.inherited_association_ends(id)?;
        let mut removed = BTreeSet::new();
        let mut replacements: BTreeMap<PropertyId, BTreeSet<PropertyId>> = BTreeMap::new();
        for &p in &candidates {
            let mut pending: Vec<_> = self.properties[&p]
                .redefines
                .iter()
                .map(|&b| (p, b))
                .collect();
            let mut visited = BTreeSet::new();
            while let Some((current, base)) = pending.pop() {
                if !candidates.contains(&base) {
                    continue;
                }
                self.interpret_association_redefinition(current, base, id)?;
                if visited.insert(base) {
                    removed.insert(base);
                    replacements.entry(p).or_default().insert(base);
                    pending.extend(self.properties[&base].redefines.iter().map(|&b| (base, b)));
                }
            }
        }
        let effective: BTreeSet<_> = candidates.difference(&removed).copied().collect();
        let mut targets = BTreeSet::new();
        for &p in &effective {
            for &base in replacements.get(&p).into_iter().flatten() {
                if !targets.insert(base) {
                    return Err(MetamodelError::UnsupportedAssociationRedefinition {
                        property: p,
                        base,
                        association: id,
                    });
                }
            }
        }
        Ok(effective)
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

    /// Generic occurrence storage is exclusive with authored class-slot storage.
    /// Derived-only associations remain descriptor metadata, with no authored links.
    pub fn supports_occurrence_storage(&self, id: AssociationId) -> Result<bool, MetamodelError> {
        let a = self.association(id)?;
        let mut authored = false;
        for &end in &a.member_ends {
            let p = self.property(end)?;
            if !p.derived && self.is_navigable(end)? {
                authored = true;
                if self.supports_slot_storage(end)? || self.inverse_storage(end)?.is_some() {
                    return Ok(false);
                }
            }
        }
        Ok(authored)
    }

    /// Occurrences produced by semantic derivation may realize derived-only
    /// associations. They remain exclusive with an authored canonical slot
    /// carrier; provenance does not permit a second store for the same links.
    pub fn supports_derived_occurrence_storage(
        &self,
        id: AssociationId,
    ) -> Result<bool, MetamodelError> {
        for &end in &self.association(id)?.member_ends {
            let p = self.property(end)?;
            if !p.derived
                && self.is_navigable(end)?
                && (self.supports_slot_storage(end)? || self.inverse_storage(end)?.is_some())
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// All raw properties in identity order, including association-owned ends.
    pub fn properties(&self) -> impl Iterator<Item = &PropertyDescriptor> {
        self.properties.values()
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
        let properties = self.effective_properties(class)?;
        // Most navigation requests already use an effective slot identity. The
        // immutable registry has its membership index; do not allocate and scan
        // every descriptor on each property read.
        if self.effective[&class].contains(&property) {
            return Ok(Some(&self.properties[&property]));
        }
        let mut matches = properties.filter(|p| self.redefined[&p.id].contains(&property));
        let first = matches.next();
        if let (Some(first), Some(second)) = (first, matches.next()) {
            return Err(MetamodelError::PropertyConflict {
                class,
                first: first.id,
                second: second.id,
            });
        }
        Ok(first)
    }

    /// Resolve a display name only after inheritance and redefinition validation.
    pub fn property_named(
        &self,
        class: MetaclassId,
        name: &str,
    ) -> Result<Option<&PropertyDescriptor>, MetamodelError> {
        let mut matches = self.effective_properties(class)?.filter(|p| p.name == name);
        let first = matches.next();
        if let (Some(first), Some(second)) = (first, matches.next()) {
            return Err(MetamodelError::PropertyConflict {
                class,
                first: first.id,
                second: second.id,
            });
        }
        Ok(first)
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
    /// Registered metaclasses in deterministic identity order.
    pub fn classes(&self) -> impl Iterator<Item = &MetaclassDescriptor> {
        self.classes.values()
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
        if let Some(error) = self.effective_errors.get(&class) {
            return Err(error.clone());
        }
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
    /// Applicability of a navigation/result without manufacturing a class slot.
    /// Association-owned properties use the exact opposite type as their context.
    pub fn is_applicable_navigation(
        &self,
        class: MetaclassId,
        property: PropertyId,
    ) -> Result<bool, MetamodelError> {
        let p = self.property(property)?;
        match p.owner {
            PropertyOwner::Class(_) => self.is_legal(class, property),
            PropertyOwner::Association(_) => self.is_subtype(class, self.property_context(p)?),
        }
    }

    /// Whether an instance may carry this property (ignoring derived write policy).
    pub fn is_legal(
        &self,
        class: MetaclassId,
        property: PropertyId,
    ) -> Result<bool, MetamodelError> {
        self.property(property)?;
        self.class(class)?;
        let _ = self.effective_properties(class)?;
        Ok(self.effective[&class].contains(&property))
    }
    pub(crate) fn supertypes(&self, class: MetaclassId) -> impl Iterator<Item = MetaclassId> + '_ {
        self.ancestors.get(&class).into_iter().flatten().copied()
    }
}
