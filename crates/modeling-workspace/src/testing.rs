//! Verification-only scheduler and storage observations; no acceptance authority.
use crate::*;

/// Whether two compilations share the same authenticated immutable dependency
/// mount. This storage observation grants no semantic or validation authority.
pub fn shares_accepted_dependency(
    left: &WorkingProjectRevision,
    right: &WorkingProjectRevision,
) -> bool {
    left.compilation
        .shares_accepted_dependency_with(&right.compilation)
}

/// Full reconstruction oracle over identical source and identity inputs.
pub fn full_rebuild(
    revision: &WorkingProjectRevision,
) -> Result<Arc<WorkingProjectRevision>, WorkspaceError> {
    Ok(Arc::new(WorkingProjectRevision {
        revision: ProjectRevision {
            revision: revision.revision(),
            parent: revision.parent(),
            compilation: revision.compilation.full_rebuild()?,
        },
    }))
}

/// Subjects recorded at actual evaluation sites, including reconstruction passes.
pub fn producer_subjects(
    revision: &WorkingProjectRevision,
) -> impl Iterator<Item = ElementId> + '_ {
    revision.compilation.observed_producer_subjects()
}

/// Physical publication table identities, including its nested KerML dependency.
pub fn publication_storage(
    publication: &CanonicalSysmlSystemsLibrary,
) -> std::collections::BTreeSet<agq_kernel::storage_observer::StorageTableIdentity> {
    agq_kernel::storage_observer::publication_storage(publication.overlay())
}

/// Storage retained by this exact current-source compilation, including Working states.
pub fn dependency_storage(
    revision: &WorkingProjectRevision,
) -> agq_kernel::storage_observer::DependencyStorage {
    revision.compilation.dependency_storage()
}
