//! Complete SysML 2.0 structural descriptors over the shared KerML 1.0 identities.
//! No SysML semantic rule execution or parsing is provided by this crate.
#![forbid(unsafe_code)]
#[path = "generated/complete.rs"]
mod generated;
#[path = "generated/typed_views.rs"]
mod generated_views;
#[doc(hidden)]
pub use agq_kerml::view;
pub use agq_kerml::{TypedView, Values, ViewError};
pub use generated::{CLASS_IDS, PROPERTY_IDS};
pub use generated_views::{classes, metamodel, properties, views};
/// SysML-owned descriptors only. Their references reuse KerML descriptor identities.
pub fn own_descriptors() -> agq_kernel::metamodel::DescriptorSet {
    generated::descriptors()
}
/// Dependency-closed graph, importing the KerML descriptors once.
pub fn descriptors() -> agq_kernel::metamodel::DescriptorSet {
    let mut all = agq_kerml::descriptors();
    let own = own_descriptors();
    all.models.extend(own.models);
    all.classes.extend(own.classes);
    all.properties.extend(own.properties);
    all.associations.extend(own.associations);
    all.enumerations.extend(own.enumerations);
    all.sources.extend(own.sources);
    all.reviews.extend(own.reviews);
    all.primitives.extend(own.primitives);
    all
}
/// Atomic structural registration. Use `validate_conformance` for authoring diagnostics.
pub fn registry()
-> Result<agq_kernel::metamodel::MetamodelRegistry, agq_kernel::metamodel::MetamodelError> {
    agq_kernel::metamodel::MetamodelRegistry::from_descriptors(descriptors())
}
