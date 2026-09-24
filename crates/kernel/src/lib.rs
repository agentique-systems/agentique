//! A metamodel-driven semantic substrate, independent of language front ends.
//!
//! This crate supplies structural invariants, not KerML/SysML semantic conformance.
//! See `docs/semantic-kernel.md` and ADR 0001 in the repository.
//!
//! A minimal programmatic transaction (an illustrative metamodel):
//! ```
//! use agq_kernel::{ElementId, MetaclassId, MetamodelId, Snapshot};
//! use agq_kernel::metamodel::*;
//! use agq_kernel::provenance::DeclaredOrigin;
//! use std::{collections::BTreeSet, sync::Arc};
//! let model_id = MetamodelId::new();
//! let class_id = MetaclassId::new();
//! let registry = MetamodelRegistry::new(
//!     [MetamodelDescriptor {
//!         id: model_id, name: "Example".into(),
//!         version: Version { major: 1, minor: 0, patch: 0 },
//!         uri: "urn:example:1".into(),
//!     }],
//!     [MetaclassDescriptor {
//!         id: class_id, name: "Node".into(), package: vec![], metamodel: model_id,
//!         direct_supertypes: BTreeSet::new(), is_abstract: false,
//!     }], [],
//! )?;
//! let before = Snapshot::new(Arc::new(registry));
//! let id = ElementId::new();
//! let mut changes = before.change_set();
//! changes.create(id, class_id, DeclaredOrigin::Authored { source: None });
//! let after = before.apply(&changes)?;
//! assert!(before.model().element(id).is_none());
//! assert_eq!(after.model().element(id).unwrap().id(), id);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
#![forbid(unsafe_code)]

pub mod association;
pub mod derived;
mod ids;
pub mod metamodel;
mod model;
pub mod numeric;
pub mod provenance;
mod shared_map;
pub mod value;

pub use ids::*;
pub use model::archive;
pub use model::{
    ChangeSet, ConstructionObligation, ConstructionView, DeclaredConstructionHistory,
    DeclaredIdentityCheckpoint, DeclaredIdentitySet, ElementRecord, ModelError, ModelView,
    ReferenceCarrier, ReferenceOccurrence, Slot, SlotShape, Snapshot,
};

#[cfg(any(test, feature = "verification"))]
pub use model::storage_observer;
