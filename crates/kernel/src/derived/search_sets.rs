//! Immutable sharing for exact structural search populations.
use super::StructuralSearch;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// Interns equal negative/structural search sets without changing their contents.
/// Allocation identities are only a hashing fast path, never semantic identities.
#[derive(Clone, Debug, Default)]
pub struct StructuralSearchPool {
    entries: HashSet<Arc<BTreeSet<StructuralSearch>>>,
    // Every indexed allocation stays alive through `entries`.
    allocations: HashSet<usize>,
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
        if self.allocations.contains(&allocation) {
            self.statistics.reused += 1;
            return searches;
        }
        if let Some(existing) = self.entries.get(&searches) {
            self.statistics.reused += 1;
            existing.clone()
        } else {
            self.allocations.insert(allocation);
            self.entries.insert(searches.clone());
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
