//! Verification-only observations of actual retained immutable table allocations.
//!
//! Addresses are process-local comparison tokens, never semantic or persisted IDs.
use super::*;
use crate::derived::{ConstructionOverlay, DerivedOverlay};

/// One actual backing table allocation, scoped to its table role.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StorageTableIdentity {
    kind: &'static str,
    address: usize,
}
/// Counts retained authoritative rows copied from immutable dependencies.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CopiedDependencyEntries {
    /// Entries by physical table category. Zero categories are omitted.
    pub tables: BTreeMap<&'static str, usize>,
}
impl CopiedDependencyEntries {
    /// True only when every observed authoritative duplication count is zero.
    pub fn is_zero(&self) -> bool {
        self.tables.values().all(|count| *count == 0)
    }
    pub(crate) fn add(&mut self, kind: &'static str, count: usize) {
        if count != 0 {
            *self.tables.entry(kind).or_default() += count;
        }
    }
}
/// Read-only storage observation for a working or validated authored state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencyStorage {
    /// The actual inherited allocations retained by this state.
    pub base_tables: BTreeSet<StorageTableIdentity>,
    /// Retained dependency-owned authoritative entries in local tables.
    pub copied_dependency_entries: CopiedDependencyEntries,
    /// Sparse local indexes/projections supported by local carriers; these are
    /// deliberately separate from copied canonical or proof/search rows.
    pub local_projection_entries: BTreeMap<&'static str, usize>,
    observed_local_tables: BTreeSet<StorageTableIdentity>,
}
pub(crate) fn table(
    tables: &mut BTreeSet<StorageTableIdentity>,
    kind: &'static str,
    address: usize,
) {
    tables.insert(StorageTableIdentity { kind, address });
}
pub(crate) fn map_tables<K: Ord, V>(
    map: &SharedMap<K, V>,
    kind: &'static str,
    tables: &mut BTreeSet<StorageTableIdentity>,
) {
    table(tables, kind, map.local_table_id());
    if let Some(base) = map.base() {
        map_tables(base, kind, tables);
    }
}
pub(crate) fn map_observation<K: Ord, V>(
    map: &SharedMap<K, V>,
    kind: &'static str,
    out: &mut DependencyStorage,
) {
    if let Some(base) = map.base() {
        map_tables(base, kind, &mut out.base_tables);
        if out.observed_local_tables.insert(StorageTableIdentity {
            kind,
            address: map.local_table_id(),
        }) {
            out.copied_dependency_entries.add(
                kind,
                map.local_iter()
                    .filter(|(key, _)| base.contains_key(key))
                    .count(),
            );
        }
    }
}
pub(crate) fn authoritative_map_observation<K: Ord, V>(
    map: &SharedMap<K, V>,
    expected: Option<&SharedMap<K, V>>,
    kind: &'static str,
    out: &mut DependencyStorage,
) {
    if let Some(base) = map.base() {
        map_tables(base, kind, &mut out.base_tables);
    }
    if let Some(expected) = expected
        && out.observed_local_tables.insert(StorageTableIdentity {
            kind,
            address: map.local_table_id(),
        })
    {
        out.copied_dependency_entries.add(
            kind,
            map.local_iter()
                .filter(|(key, _)| expected.contains_key(key))
                .count(),
        );
    }
}
fn index_tables(indexes: &Arc<Indexes>, tables: &mut BTreeSet<StorageTableIdentity>) {
    // Each owned index table resides in this immutable Indexes allocation.
    for (kind, address) in [
        ("incidence", &indexes.incidence as *const _ as usize),
        ("exact_class", &indexes.exact_class as *const _ as usize),
        ("by_supertype", &indexes.by_supertype as *const _ as usize),
        ("incoming", &indexes.incoming as *const _ as usize),
        (
            "incoming_by_property",
            &indexes.incoming_by_property as *const _ as usize,
        ),
        ("outgoing", &indexes.outgoing as *const _ as usize),
        (
            "replaced_link_groups",
            &indexes.replaced_link_groups as *const _ as usize,
        ),
    ] {
        table(tables, kind, address);
    }
    map_tables(
        &indexes.association_navigation,
        "association_navigation",
        tables,
    );
    map_tables(&indexes.inverse_slots, "inverse_slots", tables);
    if let Some(base) = &indexes.base {
        index_tables(base, tables);
    }
}
pub(crate) fn model_tables(model: &ModelView, tables: &mut BTreeSet<StorageTableIdentity>) {
    map_tables(&model.records, "records", tables);
    map_tables(&model.links, "links", tables);
    map_tables(&model.derived_navigation, "derived_navigation", tables);
    map_tables(&model.statuses, "statuses", tables);
    map_tables(&model.searches, "searches", tables);
    map_tables(
        &model.reference_contributions,
        "reference_contributions",
        tables,
    );
    index_tables(&model.indexes, tables);
}
pub(crate) fn model_observation(
    model: &ModelView,
    expected: Option<&ModelView>,
) -> DependencyStorage {
    let mut out = DependencyStorage::default();
    observe_model(model, expected, &mut out);
    out
}
pub(crate) fn observe_model(
    model: &ModelView,
    expected: Option<&ModelView>,
    out: &mut DependencyStorage,
) {
    authoritative_map_observation(&model.records, expected.map(|m| &m.records), "records", out);
    authoritative_map_observation(&model.links, expected.map(|m| &m.links), "links", out);
    authoritative_map_observation(
        &model.derived_navigation,
        expected.map(|m| &m.derived_navigation),
        "derived_navigation",
        out,
    );
    authoritative_map_observation(
        &model.statuses,
        expected.map(|m| &m.statuses),
        "statuses",
        out,
    );
    authoritative_map_observation(
        &model.searches,
        expected.map(|m| &m.searches),
        "searches",
        out,
    );
    authoritative_map_observation(
        &model.reference_contributions,
        expected.map(|m| &m.reference_contributions),
        "reference_contributions",
        out,
    );
    if let Some(base) = &model.indexes.base {
        index_tables(base, &mut out.base_tables);
        let projected_refs = model.indexes.outgoing.values().flatten().filter(|r| {
            matches!(r.carrier, ReferenceCarrier::AssociationOccurrence(id) if model.links.base().is_some_and(|links| links.contains_key(&id)))
        }).count();
        out.local_projection_entries
            .insert("inherited_link_reprojections", projected_refs);
        out.local_projection_entries.insert(
            "local_incoming_to_dependency",
            model
                .indexes
                .incoming
                .iter()
                .filter(|(id, _)| {
                    model
                        .records
                        .base()
                        .is_some_and(|records| records.contains_key(id))
                })
                .map(|(_, refs)| refs.len())
                .sum(),
        );
        out.local_projection_entries.insert(
            "navigation_overrides",
            model
                .indexes
                .inverse_slots
                .local_iter()
                .filter(|(key, _)| base.inverse_slots.contains_key(key))
                .count(),
        );
    }
}
pub(crate) fn reservation_observation(
    ids: &SharedSet<ElementId>,
    links: &SharedSet<AssociationOccurrenceId>,
    out: &mut DependencyStorage,
) {
    map_observation(&ids.0, "element_reservations", out);
    map_observation(&links.0, "occurrence_reservations", out);
}
/// All physical graph, index, proof/search and identity tables of a publication,
/// including its immutable dependency chain.
pub fn publication_storage(publication: &DerivedOverlay) -> BTreeSet<StorageTableIdentity> {
    publication.storage_tables()
}
/// Actual dependency sharing of a strict derived state.
pub fn overlay_storage(overlay: &DerivedOverlay) -> DependencyStorage {
    overlay.storage_observation()
}
/// Actual dependency sharing of an unpublished derived state.
pub fn construction_storage(overlay: &ConstructionOverlay) -> DependencyStorage {
    overlay.storage_observation()
}
/// Actual dependency sharing before producer evaluation on a strict source.
pub fn snapshot_storage(snapshot: &Snapshot) -> DependencyStorage {
    let mut out = model_observation(
        snapshot.model(),
        snapshot.immutable_dependency().map(|d| d.model()),
    );
    reservation_observation(
        &snapshot.inner.used_ids,
        &snapshot.inner.used_links,
        &mut out,
    );
    if let Some(dependency) = snapshot.immutable_dependency() {
        dependency.storage_proof_tables(&mut out.base_tables);
    }
    out
}
/// Actual dependency sharing before producer evaluation on a working source.
pub fn declared_construction_storage(candidate: &ConstructionView) -> DependencyStorage {
    let mut out = model_observation(
        candidate.model(),
        candidate.immutable_dependency().map(|d| d.model()),
    );
    reservation_observation(&candidate.used_ids, &candidate.used_links, &mut out);
    if let Some(dependency) = candidate.immutable_dependency() {
        dependency.storage_proof_tables(&mut out.base_tables);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn observer_detects_copies_even_if_the_base_link_was_flattened_away() {
        let accepted: SharedMap<_, _> = [(1u8, 2u8)].into_iter().collect();
        let copied = accepted.iter().map(|(&key, &value)| (key, value)).collect();
        let mut out = DependencyStorage::default();
        authoritative_map_observation(&copied, Some(&accepted), "records", &mut out);
        assert_eq!(out.copied_dependency_entries.tables["records"], 1);
        assert!(out.base_tables.is_empty());
        authoritative_map_observation(&copied, Some(&accepted), "records", &mut out);
        assert_eq!(
            out.copied_dependency_entries.tables["records"], 1,
            "one retained allocation observed through two views is counted once"
        );
    }
}
