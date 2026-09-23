//! Immutable sharing for exact structural search populations.
use super::StructuralSearch;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// Interns equal negative/structural search sets without changing their contents.
/// Allocation identities are only a hashing fast path, never semantic identities.
#[derive(Clone, Debug, Default)]
pub struct StructuralSearchPool {
    base: Option<Arc<StructuralSearchPool>>,
    entries: Arc<HashSet<Arc<BTreeSet<StructuralSearch>>>>,
    // Every indexed allocation stays alive through `entries`.
    allocations: Arc<HashSet<usize>>,
    statistics: StructuralSearchPoolStatistics,
}

/// Requests and retained storage for one immutable-search interning pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructuralSearchPoolStatistics {
    pub interned: usize,
    pub reused: usize,
    /// Sum of set sizes across retained distinct sets, not logical fact reads.
    pub entries: usize,
}

impl StructuralSearchPool {
    pub(crate) fn fork(&self) -> Self {
        Self {
            base: Some(Arc::new(self.clone())),
            statistics: self.statistics,
            ..Default::default()
        }
    }
    fn contains_allocation(&self, allocation: usize) -> bool {
        self.allocations.contains(&allocation)
            || self
                .base
                .as_ref()
                .is_some_and(|base| base.contains_allocation(allocation))
    }
    fn find(
        &self,
        searches: &Arc<BTreeSet<StructuralSearch>>,
    ) -> Option<&Arc<BTreeSet<StructuralSearch>>> {
        self.entries
            .get(searches)
            .or_else(|| self.base.as_ref()?.find(searches))
    }

    pub fn statistics(&self) -> StructuralSearchPoolStatistics {
        self.statistics
    }
    pub fn intern(
        &mut self,
        searches: BTreeSet<StructuralSearch>,
    ) -> Arc<BTreeSet<StructuralSearch>> {
        self.intern_shared(Arc::new(searches))
    }
    pub fn intern_shared(
        &mut self,
        searches: Arc<BTreeSet<StructuralSearch>>,
    ) -> Arc<BTreeSet<StructuralSearch>> {
        let allocation = Arc::as_ptr(&searches) as usize;
        if self.contains_allocation(allocation) {
            self.statistics.reused += 1;
            return searches;
        }
        if let Some(existing) = self.find(&searches).cloned() {
            self.statistics.reused += 1;
            existing
        } else {
            Arc::make_mut(&mut self.allocations).insert(allocation);
            Arc::make_mut(&mut self.entries).insert(searches.clone());
            self.statistics.interned += 1;
            self.statistics.entries += searches.len();
            searches
        }
    }
    /// Exact union. Equal or contained inputs reuse an allocation; callers'
    /// immutable sets are never changed when additional reads are required.
    pub fn union_shared(
        &mut self,
        previous: &Arc<BTreeSet<StructuralSearch>>,
        additional: Arc<BTreeSet<StructuralSearch>>,
    ) -> Arc<BTreeSet<StructuralSearch>> {
        let previous = self.intern_shared(previous.clone());
        let additional = self.intern_shared(additional);
        if Arc::ptr_eq(&previous, &additional) || additional.is_subset(&previous) {
            previous
        } else if previous.is_subset(&additional) {
            additional
        } else {
            self.intern(previous.union(&additional).cloned().collect())
        }
    }
}

#[cfg(any(test, feature = "verification"))]
impl StructuralSearchPool {
    pub(crate) fn storage_tables(
        &self,
        out: &mut BTreeSet<crate::storage_observer::StorageTableIdentity>,
    ) {
        use crate::storage_observer::table;
        table(out, "search_intern", Arc::as_ptr(&self.entries) as usize);
        table(
            out,
            "search_allocations",
            Arc::as_ptr(&self.allocations) as usize,
        );
        if let Some(base) = &self.base {
            base.storage_tables(out);
        }
    }
    pub(crate) fn storage_observation(&self, out: &mut crate::storage_observer::DependencyStorage) {
        if let Some(base) = &self.base {
            base.storage_tables(&mut out.base_tables);
            out.copied_dependency_entries.add(
                "search_intern",
                self.entries
                    .iter()
                    .filter(|searches| base.find(searches).is_some())
                    .count(),
            );
            out.copied_dependency_entries.add(
                "search_allocations",
                self.allocations
                    .iter()
                    .filter(|id| base.contains_allocation(**id))
                    .count(),
            );
        }
    }
}
