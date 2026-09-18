//! Borrowed views and checked property projection. No canonical data lives here.
use agq_kernel::{
    ElementId, ElementRecord, EnumerationLiteralId, MetaclassId, ModelView, PropertyId, SlotShape,
    metamodel::{MetamodelError, PropertyDescriptor, ValueKind},
    value::{SlotValue, Value},
};
use std::{fmt, marker::PhantomData};

/// A failed cast or property read. Missing derivations are never empty values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewError {
    ComputationFailure {
        element: ElementId,
        property: PropertyId,
        failure: Box<agq_kernel::derived::ComputationFailure>,
    },
    Registry(MetamodelError),
    UnknownElement(ElementId),
    WrongClass {
        element: ElementId,
        actual: MetaclassId,
        expected: MetaclassId,
    },
    IllegalProperty {
        element: ElementId,
        property: PropertyId,
    },
    NotComputed {
        element: ElementId,
        property: PropertyId,
    },
    UnsupportedAssociationStorage(PropertyId),
    MissingRequired {
        element: ElementId,
        property: PropertyId,
    },
    IncompatibleProperty {
        property: PropertyId,
        expected: ValueKind,
        actual: ValueKind,
    },
    MalformedSlot {
        element: ElementId,
        property: PropertyId,
        reason: &'static str,
    },
}
impl From<MetamodelError> for ViewError {
    fn from(value: MetamodelError) -> Self {
        Self::Registry(value)
    }
}
impl fmt::Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComputationFailure {
                element,
                property,
                failure,
            } => write!(f, "derived property {element}/{property}: {failure:?}"),
            Self::Registry(error) => error.fmt(f),
            Self::UnknownElement(id) => write!(f, "unknown element {id}"),
            Self::WrongClass {
                element,
                actual,
                expected,
            } => write!(
                f,
                "element {element} has class {actual}, expected an instance/subtype of {expected}"
            ),
            Self::IllegalProperty { element, property } => {
                write!(f, "property {property} is not applicable to {element}")
            }
            Self::NotComputed { element, property } => write!(
                f,
                "derived property {element}/{property} has not been computed"
            ),
            Self::UnsupportedAssociationStorage(property) => write!(
                f,
                "property {property} requires canonical association storage"
            ),
            Self::MissingRequired { element, property } => {
                write!(f, "required property {element}/{property} is missing")
            }
            Self::IncompatibleProperty {
                property,
                expected,
                actual,
            } => write!(
                f,
                "property {property} has domain {actual:?}, expected {expected:?}"
            ),
            Self::MalformedSlot {
                element,
                property,
                reason,
            } => write!(f, "malformed slot {element}/{property}: {reason}"),
        }
    }
}
impl std::error::Error for ViewError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Registry(error) => Some(error),
            _ => None,
        }
    }
}

#[doc(hidden)]
pub mod sealed {
    pub trait Sealed {}
}

/// Common capability of generated views; implementations are sealed and checked.
/// Equality of semantic identity is equality of `id()`, even across snapshots.
pub trait TypedView<'m>: sealed::Sealed + Copy {
    const CLASS: MetaclassId;
    fn try_new(id: ElementId, model: &'m ModelView) -> Result<Self, ViewError>;
    fn id(self) -> ElementId;
    fn model(self) -> &'m ModelView;
    fn record(self) -> &'m ElementRecord {
        self.model()
            .element(self.id())
            .expect("checked immutable view")
    }
    fn cast<T: TypedView<'m>>(self) -> Result<T, ViewError> {
        T::try_new(self.id(), self.model())
    }
}

#[doc(hidden)]
pub fn check(id: ElementId, model: &ModelView, expected: MetaclassId) -> Result<(), ViewError> {
    let record = model.element(id).ok_or(ViewError::UnknownElement(id))?;
    if !model.registry().is_subtype(record.metaclass(), expected)? {
        return Err(ViewError::WrongClass {
            element: id,
            actual: record.metaclass(),
            expected,
        });
    }
    Ok(())
}

#[macro_export]
macro_rules! define_view {
    ($name:ident, $class:expr) => {
        /// Checked borrowed access to one kernel record. Stores only identity and model reference.
        #[derive(Clone, Copy, Debug)]
        pub struct $name<'m> {
            id: agq_kernel::ElementId,
            model: &'m agq_kernel::ModelView,
        }
        impl $crate::view::sealed::Sealed for $name<'_> {}
        impl<'m> $crate::TypedView<'m> for $name<'m> {
            const CLASS: agq_kernel::MetaclassId = $class;
            fn try_new(
                id: agq_kernel::ElementId,
                model: &'m agq_kernel::ModelView,
            ) -> Result<Self, $crate::ViewError> {
                $crate::view::check(id, model, Self::CLASS)?;
                Ok(Self { id, model })
            }
            fn id(self) -> agq_kernel::ElementId {
                self.id
            }
            fn model(self) -> &'m agq_kernel::ModelView {
                self.model
            }
        }
        impl<'m> $name<'m> {
            pub const CLASS: agq_kernel::MetaclassId = $class;
            pub fn try_new(
                id: agq_kernel::ElementId,
                model: &'m agq_kernel::ModelView,
            ) -> Result<Self, $crate::ViewError> {
                <Self as $crate::TypedView>::try_new(id, model)
            }
            pub fn id(self) -> agq_kernel::ElementId {
                self.id
            }
            pub fn model(self) -> &'m agq_kernel::ModelView {
                self.model
            }
            pub fn record(self) -> &'m agq_kernel::ElementRecord {
                $crate::TypedView::record(self)
            }
            pub fn cast<T: $crate::TypedView<'m>>(self) -> Result<T, $crate::ViewError> {
                $crate::TypedView::cast(self)
            }
        }
    };
}
#[doc(hidden)]
pub use crate::define_view;

// Only these projections can instantiate Values. They borrow text and copy IDs/primitives.
#[doc(hidden)]
pub trait Decode<'m>: Copy {
    fn decode(value: &'m Value) -> Option<Self>;
}
impl<'m> Decode<'m> for &'m str {
    fn decode(value: &'m Value) -> Option<Self> {
        if let Value::String(v) = value {
            Some(v)
        } else {
            None
        }
    }
}
macro_rules! decode {
    ($ty:ty, $variant:ident) => {
        impl<'m> Decode<'m> for $ty {
            fn decode(value: &'m Value) -> Option<Self> {
                if let Value::$variant(v) = value {
                    Some(*v)
                } else {
                    None
                }
            }
        }
    };
}
decode!(bool, Boolean);
impl<'m> Decode<'m> for &'m agq_kernel::numeric::Integer {
    fn decode(value: &'m Value) -> Option<Self> {
        if let Value::Integer(v) = value {
            Some(v)
        } else {
            None
        }
    }
}
impl<'m> Decode<'m> for &'m agq_kernel::numeric::ExactDecimal {
    fn decode(value: &'m Value) -> Option<Self> {
        if let Value::Real(v) = value {
            Some(v)
        } else {
            None
        }
    }
}
decode!(ElementId, Reference);
decode!(EnumerationLiteralId, Enumeration);

/// Allocation-free typed access to the original slot. `None` from a collection
/// accessor means absent; `Some` with `is_empty()` means a present empty collection.
/// Unordered iteration is deterministic storage order, never semantic order.
/// A supertype collection may resolve to a scalar redefinition; `shape()` reports it.
#[derive(Clone, Copy, Debug)]
pub struct Values<'m, T> {
    value: &'m SlotValue,
    descriptor: &'m PropertyDescriptor,
    marker: PhantomData<T>,
}
impl<'m, T> Values<'m, T> {
    pub fn raw(&self) -> &'m SlotValue {
        self.value
    }
    /// Effective property, after registry redefinition resolution.
    pub fn descriptor(&self) -> &'m PropertyDescriptor {
        self.descriptor
    }
    pub fn shape(&self) -> SlotShape {
        shape(self.value)
    }
    pub fn len(&self) -> usize {
        match self.value {
            SlotValue::Scalar(_) => 1,
            SlotValue::Ordered(v) | SlotValue::Bag(v) => v.len(),
            SlotValue::Set(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
// The private bound intentionally seals the set of supported scalar projections.
#[allow(private_bounds)]
impl<'m, T: Decode<'m>> Values<'m, T> {
    pub fn iter(&self) -> impl Iterator<Item = T> + 'm + use<'m, T> {
        self.value
            .values()
            .map(|v| T::decode(v).expect("eagerly checked immutable slot"))
    }
}

fn shape(value: &SlotValue) -> SlotShape {
    match value {
        SlotValue::Scalar(_) => SlotShape::Scalar,
        SlotValue::Ordered(_) => SlotShape::Ordered,
        SlotValue::Set(_) => SlotShape::Set,
        SlotValue::Bag(_) => SlotShape::Bag,
    }
}
fn validate<'m, T: Decode<'m>>(
    id: ElementId,
    p: &PropertyDescriptor,
    value: &'m SlotValue,
) -> Result<(), ViewError> {
    let expected = if p.multiplicity.upper.is_some_and(|n| n <= 1) {
        SlotShape::Scalar
    } else if p.ordered {
        SlotShape::Ordered
    } else if p.unique {
        SlotShape::Set
    } else {
        SlotShape::Bag
    };
    let fail = |reason| ViewError::MalformedSlot {
        element: id,
        property: p.id,
        reason,
    };
    if shape(value) != expected {
        return Err(fail("collection shape disagrees with effective descriptor"));
    }
    let count = value.values().count();
    if count < p.multiplicity.lower || p.multiplicity.upper.is_some_and(|n| count > n) {
        return Err(fail("multiplicity disagrees with effective descriptor"));
    }
    if value.values().any(|v| T::decode(v).is_none()) {
        return Err(fail("value has the wrong scalar domain"));
    }
    Ok(())
}

#[doc(hidden)]
pub mod read {
    use super::*;
    fn slot<'m, T: Decode<'m>>(
        id: ElementId,
        model: &'m ModelView,
        property: PropertyId,
        kind: ValueKind,
    ) -> Result<(&'m PropertyDescriptor, Option<&'m SlotValue>), ViewError> {
        let record = model.element(id).ok_or(ViewError::UnknownElement(id))?;
        let registry = model.registry();
        let p = registry
            .resolve_property(record.metaclass(), property)?
            .ok_or(ViewError::IllegalProperty {
                element: id,
                property,
            })?;
        let compatible = match (p.value_kind, kind) {
            (ValueKind::Reference(actual), ValueKind::Reference(expected)) => {
                registry.is_subtype(actual, expected)?
            }
            (actual, expected) => actual == expected,
        };
        if !compatible {
            return Err(ViewError::IncompatibleProperty {
                property: p.id,
                actual: p.value_kind,
                expected: kind,
            });
        }
        if !registry.supports_slot_storage(p.id)?
            && registry.inverse_storage(p.id)?.is_none()
            && !p
                .association
                .is_some_and(|a| registry.supports_occurrence_storage(a).unwrap_or(false))
        {
            return Err(ViewError::UnsupportedAssociationStorage(p.id));
        }
        if let Ok(
            agq_kernel::derived::PropertyState::Incomplete(failure)
            | agq_kernel::derived::PropertyState::Invalid(failure),
        ) = model.property_state(id, p.id)
        {
            return Err(ViewError::ComputationFailure {
                element: id,
                property: p.id,
                failure: Box::new(failure.clone()),
            });
        }
        let value = model.navigation_slot(id, p.id).map(|s| s.value());
        if let Some(value) = value {
            validate::<T>(id, p, value)?;
        } else if p.derived {
            return Err(ViewError::NotComputed {
                element: id,
                property: p.id,
            });
        } else if p.multiplicity.lower > 0 {
            return Err(ViewError::MissingRequired {
                element: id,
                property: p.id,
            });
        }
        Ok((p, value))
    }
    #[doc(hidden)]
    pub fn optional<'m, T: Decode<'m>>(
        id: ElementId,
        model: &'m ModelView,
        property: PropertyId,
        kind: ValueKind,
    ) -> Result<Option<T>, ViewError> {
        let (p, value) = slot::<T>(id, model, property, kind)?;
        match value {
            None => Ok(None),
            Some(SlotValue::Scalar(v)) => T::decode(v).map(Some).ok_or(ViewError::MalformedSlot {
                element: id,
                property: p.id,
                reason: "value has the wrong scalar domain",
            }),
            Some(_) => Err(ViewError::MalformedSlot {
                element: id,
                property: p.id,
                reason: "scalar accessor resolved to a collection",
            }),
        }
    }
    #[doc(hidden)]
    pub fn required<'m, T: Decode<'m>>(
        id: ElementId,
        model: &'m ModelView,
        property: PropertyId,
        kind: ValueKind,
    ) -> Result<T, ViewError> {
        optional(id, model, property, kind)?.ok_or(ViewError::MissingRequired {
            element: id,
            property,
        })
    }
    #[doc(hidden)]
    pub fn many<'m, T: Decode<'m>>(
        id: ElementId,
        model: &'m ModelView,
        property: PropertyId,
        kind: ValueKind,
    ) -> Result<Option<Values<'m, T>>, ViewError> {
        let (descriptor, value) = slot::<T>(id, model, property, kind)?;
        Ok(value.map(|value| Values {
            value,
            descriptor,
            marker: PhantomData,
        }))
    }
    #[doc(hidden)]
    pub fn required_many<'m, T: Decode<'m>>(
        id: ElementId,
        model: &'m ModelView,
        property: PropertyId,
        kind: ValueKind,
    ) -> Result<Values<'m, T>, ViewError> {
        many(id, model, property, kind)?.ok_or(ViewError::MissingRequired {
            element: id,
            property,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::properties;
    use std::collections::BTreeSet;

    #[test]
    fn projection_rejects_malformed_shapes_and_values_without_filtering() {
        let registry = crate::registry().unwrap();
        let aliases = registry.property(properties::ELEMENT_ALIAS_IDS).unwrap();
        let id = ElementId::new();
        for value in [
            SlotValue::Scalar(Value::String("scalar".into())),
            SlotValue::Set(BTreeSet::new()),
            SlotValue::Bag(vec![]),
            SlotValue::Ordered(vec![Value::String("valid".into()), Value::Boolean(true)]),
        ] {
            assert!(matches!(
                validate::<&str>(id, aliases, &value),
                Err(ViewError::MalformedSlot { .. })
            ));
        }
        let required = registry.property(properties::ELEMENT_ELEMENT_ID).unwrap();
        let mut invalid_bounds = required.clone();
        invalid_bounds.multiplicity.upper = Some(0);
        assert!(matches!(
            validate::<&str>(
                id,
                &invalid_bounds,
                &SlotValue::Scalar(Value::String("one".into()))
            ),
            Err(ViewError::MalformedSlot { .. })
        ));
    }
}
