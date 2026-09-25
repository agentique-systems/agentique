//! Disposable completed DTOs scoped to one already authenticated bound revision.
//! This private component cannot mint a reader or authorize a semantic query.
use crate::{ElementInspector, PlatformError, Result, RevisionBinding};
use agq_kernel::ElementId;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
};

const CAPACITY: usize = 16;

#[derive(Default)]
struct Entries {
    values: BTreeMap<ElementId, Arc<ElementInspector>>,
    oldest_first: VecDeque<ElementId>,
}

impl Entries {
    fn touch(&mut self, element: ElementId) {
        self.oldest_first.retain(|id| *id != element);
        self.oldest_first.push_back(element);
    }

    fn get(&mut self, element: ElementId) -> Option<Arc<ElementInspector>> {
        let value = self.values.get(&element).cloned();
        if value.is_some() {
            self.touch(element);
        }
        value
    }

    fn insert(&mut self, element: ElementId, value: Arc<ElementInspector>) -> Result<()> {
        // A concurrent miss may have finished while this one was computing.
        // Same immutable input cannot silently replace a different completed DTO.
        if let Some(existing) = self.values.get(&element) {
            if existing.as_ref() != value.as_ref() {
                return Err(PlatformError::Invalid(
                    "Inspector results disagree for one immutable reader and element".into(),
                ));
            }
        } else {
            if self.values.len() == CAPACITY
                && let Some(oldest) = self.oldest_first.pop_front()
            {
                self.values.remove(&oldest);
            }
            self.values.insert(element, value);
        }
        self.touch(element);
        Ok(())
    }
}

pub(super) struct InspectorCache {
    binding: RevisionBinding,
    entries: Mutex<Entries>,
}

impl InspectorCache {
    pub(super) fn new(binding: RevisionBinding) -> Self {
        Self {
            binding,
            entries: Mutex::new(Entries::default()),
        }
    }

    fn check_identity(&self, element: ElementId, value: &ElementInspector) -> Result<()> {
        if value.revision_id != self.binding.revision
            || value.element.revision_id != self.binding.revision
            || value.element.id != element
        {
            return Err(PlatformError::Invalid(
                "Inspector identity does not match the immutable reader and requested element"
                    .into(),
            ));
        }
        Ok(())
    }

    /// The bool describes whether lookup found the completed DTO, not semantic
    /// completeness. A cache failure may be bypassed; it cannot authorize data.
    pub(super) fn read(
        &self,
        element: ElementId,
        compute: impl FnOnce() -> Result<ElementInspector>,
    ) -> (bool, Result<ElementInspector>) {
        let cached = self
            .entries
            .lock()
            .ok()
            .and_then(|mut entries| entries.get(element));
        if let Some(value) = cached {
            return (
                true,
                self.check_identity(element, &value)
                    .map(|()| (*value).clone()),
            );
        }
        // No mutex is held across semantic queries, failures or a panic in them.
        let value = match compute() {
            Ok(value) => value,
            Err(error) => return (false, Err(error)),
        };
        if let Err(error) = self.check_identity(element, &value) {
            return (false, Err(error));
        }
        let value = Arc::new(value);
        if let Ok(mut entries) = self.entries.lock()
            && let Err(error) = entries.insert(element, value.clone())
        {
            return (false, Err(error));
        }
        // Owned return data cannot mutate the retained entry. Clone outside the
        // mutex so another caller can look up a different completed inspector.
        (false, Ok((*value).clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_modeling_repository::{ProjectId, ProjectRevisionId};
    use agq_modeling_view::{FeatureCounts, QuerySummary, ViewNode, ViewOrigin};
    use std::sync::{
        Barrier,
        atomic::{AtomicUsize, Ordering},
    };

    fn binding() -> RevisionBinding {
        RevisionBinding {
            project: ProjectId::new(),
            revision: ProjectRevisionId::new(),
        }
    }

    // Explicit cache-mechanics DTO only. This cannot mint an authenticated
    // BoundRevision or establish any query completeness or validation claim.
    fn inspector(binding: RevisionBinding, id: ElementId) -> ElementInspector {
        ElementInspector {
            revision_id: binding.revision,
            element: ViewNode {
                id,
                revision_id: binding.revision,
                semantic_kind: "PartUsage".into(),
                name: "Unit-only inspector".into(),
                qualified_name: None,
                owner: None,
                origin: ViewOrigin::Authored,
                source_available: false,
                features: vec![],
                counts: FeatureCounts::default(),
                badges: vec![],
            },
            owner: None,
            effective_types: vec![],
            owned_features: vec![],
            effective_features: vec![],
            feature_provenance: Default::default(),
            specializations: vec![],
            subsettings: vec![],
            redefinitions: vec![],
            relationships: vec![],
            multiplicity: None,
            source: None,
            queries: vec![QuerySummary {
                name: "Unit-only incomplete query".into(),
                completeness: "Incomplete".into(),
                diagnostics: vec!["unit-only retained missing evidence".into()],
                positive_dependency_count: 2,
                search_dependency_count: 3,
            }],
            profile: "Unit-only profile; not semantic acceptance".into(),
        }
    }

    #[test]
    fn completed_incomplete_dto_is_exact_and_returned_clones_cannot_change_it() {
        let binding = binding();
        let cache = InspectorCache::new(binding);
        let id = ElementId::from_u128(1);
        let expected = inspector(binding, id);
        let calls = AtomicUsize::new(0);
        let (hit, first) = cache.read(id, || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(expected.clone())
        });
        assert!(!hit);
        assert_eq!(first.unwrap(), expected);
        let (hit, second) = cache.read(id, || panic!("cache hit computed again"));
        assert!(hit);
        let mut returned = second.unwrap();
        assert_eq!(returned, expected);
        returned.element.name.push_str(" changed by caller");
        returned.queries[0].completeness = "caller changed only its clone".into();
        let (hit, third) = cache.read(id, || panic!("caller changed the cached DTO"));
        assert!(hit);
        assert_eq!(third.unwrap(), expected);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn failures_are_retried_and_never_become_negative_cache_entries() {
        let binding = binding();
        let cache = InspectorCache::new(binding);
        let id = ElementId::from_u128(2);
        for _ in 0..2 {
            let (hit, result) =
                cache.read(id, || Err(PlatformError::Invalid("query failed".into())));
            assert!(!hit);
            assert!(result.is_err());
            assert!(cache.entries.lock().unwrap().values.is_empty());
        }
        let expected = inspector(binding, id);
        let (hit, result) = cache.read(id, || Ok(expected.clone()));
        assert!(!hit);
        assert_eq!(result.unwrap(), expected);
    }

    #[test]
    fn wrong_element_or_outer_or_nested_revision_is_never_cached() {
        for wrong in 0..3 {
            let binding = binding();
            let cache = InspectorCache::new(binding);
            let id = ElementId::from_u128(3);
            let mut invalid = inspector(binding, id);
            match wrong {
                0 => invalid.element.id = ElementId::from_u128(4),
                1 => invalid.revision_id = ProjectRevisionId::new(),
                _ => invalid.element.revision_id = ProjectRevisionId::new(),
            }
            let (hit, result) = cache.read(id, || Ok(invalid));
            assert!(!hit);
            assert!(result.is_err());
            assert!(cache.entries.lock().unwrap().values.is_empty());
            let expected = inspector(binding, id);
            assert_eq!(cache.read(id, || Ok(expected.clone())).1.unwrap(), expected);
        }
    }

    #[test]
    fn fixed_bound_evicts_the_least_recently_used_completed_entry() {
        let binding = binding();
        let cache = InspectorCache::new(binding);
        for i in 0..CAPACITY {
            let id = ElementId::from_u128(i as u128);
            assert!(!cache.read(id, || Ok(inspector(binding, id))).0);
        }
        let first = ElementId::from_u128(0);
        assert!(cache.read(first, || panic!("first entry absent")).0);
        let extra = ElementId::from_u128(CAPACITY as u128);
        assert!(!cache.read(extra, || Ok(inspector(binding, extra))).0);
        let entries = cache.entries.lock().unwrap();
        assert_eq!(entries.values.len(), CAPACITY);
        assert_eq!(entries.oldest_first.len(), CAPACITY);
        assert!(entries.values.contains_key(&first));
        assert!(!entries.values.contains_key(&ElementId::from_u128(1)));
        drop(entries);
        let evicted = ElementId::from_u128(1);
        assert!(!cache.read(evicted, || Ok(inspector(binding, evicted))).0);
        assert_eq!(cache.entries.lock().unwrap().values.len(), CAPACITY);
    }

    #[test]
    fn equal_element_ids_in_different_reader_bindings_do_not_share_entries() {
        let a = binding();
        let b = RevisionBinding {
            revision: ProjectRevisionId::new(),
            ..a
        };
        let first = InspectorCache::new(a);
        let second = InspectorCache::new(b);
        let id = ElementId::from_u128(5);
        let expected_a = inspector(a, id);
        let expected_b = inspector(b, id);
        assert!(!first.read(id, || Ok(expected_a.clone())).0);
        assert!(!second.read(id, || Ok(expected_b.clone())).0);
        assert_ne!(expected_a, expected_b);
        assert_eq!(
            first.read(id, || panic!("reader A missed")).1.unwrap(),
            expected_a
        );
        assert_eq!(
            second.read(id, || panic!("reader B missed")).1.unwrap(),
            expected_b
        );
    }

    #[test]
    fn concurrent_duplicate_completions_compute_outside_lock_and_retain_one_exact_entry() {
        let binding = binding();
        let cache = Arc::new(InspectorCache::new(binding));
        let barrier = Arc::new(Barrier::new(2));
        let id = ElementId::from_u128(6);
        let expected = inspector(binding, id);
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let cache = cache.clone();
                let barrier = barrier.clone();
                let expected = expected.clone();
                std::thread::spawn(move || {
                    cache.read(id, || {
                        // This would deadlock if semantic work held the cache mutex.
                        barrier.wait();
                        Ok(expected)
                    })
                })
            })
            .collect();
        for worker in workers {
            let (hit, result) = worker.join().unwrap();
            assert!(!hit, "both lookups occurred before either result completed");
            assert_eq!(result.unwrap(), expected);
        }
        assert_eq!(cache.entries.lock().unwrap().values.len(), 1);
        assert_eq!(
            cache
                .read(id, || panic!("duplicate completion lost entry"))
                .1
                .unwrap(),
            expected
        );
    }

    #[test]
    fn conflicting_concurrent_completion_cannot_replace_a_completed_immutable_result() {
        let binding = binding();
        let cache = Arc::new(InspectorCache::new(binding));
        let barrier = Arc::new(Barrier::new(2));
        let id = ElementId::from_u128(7);
        let workers: Vec<_> = (0..2)
            .map(|index| {
                let cache = cache.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    cache.read(id, || {
                        let mut value = inspector(binding, id);
                        value.element.name = format!("conflicting unit completion {index}");
                        barrier.wait();
                        Ok(value)
                    })
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap().1)
            .collect();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
        let retained = results
            .into_iter()
            .find_map(std::result::Result::ok)
            .unwrap();
        assert_eq!(
            cache
                .read(id, || panic!("conflict removed first result"))
                .1
                .unwrap(),
            retained
        );
        assert_eq!(cache.entries.lock().unwrap().values.len(), 1);
    }
}
