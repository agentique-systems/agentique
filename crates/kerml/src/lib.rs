//! Generated KerML 1.0 structural descriptors, not a KerML semantic evaluator.
//! No runtime dependency on XMI, JSON Schema, the importer or external services.
#![forbid(unsafe_code)]

#[path = "generated/root_core.rs"]
mod generated;

pub use generated::{CLASS_IDS, PROPERTY_IDS, descriptors};

/// Build a validated registry from the checked-in normative descriptor slice.
pub fn registry()
-> Result<agq_kernel::metamodel::MetamodelRegistry, agq_kernel::metamodel::MetamodelError> {
    agq_kernel::metamodel::MetamodelRegistry::from_descriptors(descriptors())
}
