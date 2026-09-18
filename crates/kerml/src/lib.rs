//! KerML 1.0 descriptors and borrowed typed views over the generic kernel.
//! No runtime dependency on XMI, JSON Schema, the importer or external services.
//! Typed views are not canonical objects and do not evaluate KerML semantic rules.
//!
//! ```
//! use agq_kerml::{classes, properties, views::Feature};
//! use agq_kernel::{ElementId, Snapshot, provenance::DeclaredOrigin, value::*};
//! use std::sync::Arc;
//! let before = Snapshot::new(Arc::new(agq_kerml::registry()?));
//! let id = ElementId::new();
//! let mut changes = before.change_set();
//! let authored = DeclaredOrigin::Authored { source: None };
//! changes.create(id, classes::FEATURE, authored.clone());
//! // The generic kernel does not apply normative default-value rules.
//! for p in before.model().registry().effective_properties(classes::FEATURE)? {
//!     if !p.derived && p.multiplicity.lower > 0 {
//!         let value = match before.model().registry().storage_kind(p.value_kind)? {
//!             agq_kernel::metamodel::ValueKind::Boolean => Value::Boolean(false),
//!             agq_kernel::metamodel::ValueKind::String => Value::String("example".into()),
//!             _ => unreachable!("Feature has only required primitive slots"),
//!         };
//!         changes.set(id, p.id, SlotValue::Scalar(value), authored.clone());
//!     }
//! }
//! changes.set(id, properties::ELEMENT_DECLARED_NAME,
//!     SlotValue::Scalar(Value::String("speed".into())), authored);
//! let after = before.apply(&changes)?;
//! let feature = Feature::try_new(id, after.model())?;
//! assert_eq!(feature.declared_name()?, Some("speed"));
//! assert_eq!(feature.as_type()?.id(), id);
//! assert!(std::ptr::eq(feature.record(), after.model().element(id).unwrap()));
//! assert!(before.model().is_empty());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![forbid(unsafe_code)]

#[path = "generated/complete.rs"]
mod generated;
#[path = "generated/typed_views.rs"]
mod generated_views;
#[doc(hidden)]
pub mod view;

pub use generated::{CLASS_IDS, PROPERTY_IDS, descriptors};
pub use generated_views::{classes, metamodel, properties, views};
pub use view::{TypedView, Values, ViewError};

/// Build a validated registry from the checked-in normative descriptor graph.
pub fn registry()
-> Result<agq_kernel::metamodel::MetamodelRegistry, agq_kernel::metamodel::MetamodelError> {
    agq_kernel::metamodel::MetamodelRegistry::from_descriptors(descriptors())
}
