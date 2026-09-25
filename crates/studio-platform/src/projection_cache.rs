//! Completed presentation DTOs only; no model, query context or authority is cached.
use crate::{PlatformError, Result, RevisionBinding, ViewDefinition, ViewProjection};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const CAPACITY: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CacheAccess {
    NotConsulted,
    Hit,
    Miss,
}

struct Entry {
    binding: RevisionBinding,
    value: Arc<ViewProjection>,
}

#[derive(Default)]
struct Entries(VecDeque<Entry>);

impl Entries {
    fn position(&self, binding: RevisionBinding, view: &ViewDefinition) -> Option<usize> {
        self.0
            .iter()
            .position(|entry| entry.binding == binding && entry.value.view == *view)
    }

    fn get(
        &mut self,
        binding: RevisionBinding,
        view: &ViewDefinition,
    ) -> Option<Arc<ViewProjection>> {
        let index = self.position(binding, view)?;
        let entry = self.0.remove(index).expect("matched completed projection");
        let value = entry.value.clone();
        self.0.push_back(entry);
        Some(value)
    }

    /// The caller compares concurrent completions and releases an evicted DTO
    /// after dropping the mutex. No full DTO copy/equality/drop runs in this lock.
    fn offer(
        &mut self,
        binding: RevisionBinding,
        value: Arc<ViewProjection>,
    ) -> (Option<Arc<ViewProjection>>, Option<Entry>) {
        if let Some(existing) = self.get(binding, &value.view) {
            return (Some(existing), None);
        }
        let retired = (self.0.len() == CAPACITY)
            .then(|| self.0.pop_front())
            .flatten();
        self.0.push_back(Entry { binding, value });
        (None, retired)
    }
}

#[derive(Default)]
pub(super) struct ProjectionCache {
    entries: Mutex<Entries>,
}

fn check_identity(
    binding: RevisionBinding,
    view: &ViewDefinition,
    value: &ViewProjection,
) -> Result<()> {
    if value.revision_id != binding.revision
        || value.view != *view
        || value
            .nodes
            .iter()
            .any(|node| node.revision_id != binding.revision)
        || value
            .edges
            .iter()
            .any(|edge| edge.revision_id != binding.revision)
    {
        return Err(PlatformError::Invalid(
            "Projection identity does not match the resolved revision and exact view definition"
                .into(),
        ));
    }
    Ok(())
}

impl ProjectionCache {
    /// Resolution is deliberately before lookup, even when an entry exists.
    /// The private callback comes only from StudioPlatform's ordinary `bound`.
    /// The generic payload permits inert cache-mechanics tests without minting
    /// a BoundRevision or substituting semantic fixtures for authenticated data.
    pub(super) fn read<B>(
        &self,
        binding: RevisionBinding,
        view: &ViewDefinition,
        resolve: impl FnOnce() -> Result<(RevisionBinding, B)>,
        compute: impl FnOnce(B) -> Result<ViewProjection>,
    ) -> (CacheAccess, Result<ViewProjection>) {
        let bound = match resolve() {
            Ok((actual, bound)) if actual == binding => bound,
            Ok(_) => return (
                CacheAccess::NotConsulted,
                Err(PlatformError::Invalid(
                    "Resolved projection binding differs from the requested project or revision"
                        .into(),
                )),
            ),
            Err(error) => return (CacheAccess::NotConsulted, Err(error)),
        };
        let cached = self
            .entries
            .lock()
            .ok()
            .and_then(|mut entries| entries.get(binding, view));
        if let Some(value) = cached {
            return (
                CacheAccess::Hit,
                check_identity(binding, view, &value).map(|()| (*value).clone()),
            );
        }
        let value = match compute(bound) {
            Ok(value) => value,
            Err(error) => return (CacheAccess::Miss, Err(error)),
        };
        if let Err(error) = check_identity(binding, view, &value) {
            return (CacheAccess::Miss, Err(error));
        }
        let value = Arc::new(value);
        let (existing, retired) = self
            .entries
            .lock()
            .ok()
            .map(|mut entries| entries.offer(binding, value.clone()))
            .unwrap_or_default();
        drop(retired);
        // Both the expensive equality check and return clone occur outside the
        // lock. A disagreeing concurrent completion cannot replace the first DTO.
        if existing.is_some_and(|existing| existing.as_ref() != value.as_ref()) {
            return (
                CacheAccess::Miss,
                Err(PlatformError::Invalid(
                    "Projection results disagree for one immutable binding and view definition"
                        .into(),
                )),
            );
        }
        // A poisoned/unavailable cache merely bypasses retention. Identity checks
        // and ordinary read authorization remain mandatory for each fresh result.
        (CacheAccess::Miss, Ok((*value).clone()))
    }
}

#[cfg(test)]
#[path = "projection_cache_tests.rs"]
mod tests;
