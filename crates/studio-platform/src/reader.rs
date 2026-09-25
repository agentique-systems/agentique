use crate::*;
use std::{io::Write, time::Instant};

mod inspector_cache;
use inspector_cache::InspectorCache;

/// Read capability for one already resolved immutable revision.
///
/// Only the trusted platform can mint this handle after checking `Authority::Read`.
/// It retains the authenticated revision, not a service, repository, candidate or
/// policy that a worker could use to resolve another revision or mutate a model.
/// Retention does not invent revocation semantics: the native host separately
/// invalidates presentation access when its runtime or selected revision changes.
pub struct StudioRevisionReader {
    bound: BoundRevision,
    inspectors: InspectorCache,
}

impl StudioRevisionReader {
    /// Exact project and immutable revision authenticated when this reader was minted.
    pub fn binding(&self) -> RevisionBinding {
        RevisionBinding {
            project: self.bound.manifest().project_id,
            revision: self.bound.manifest().revision_id,
        }
    }

    /// Inspect canonical engineering meaning in the retained immutable model.
    /// Successful results are reused within this reader's bounded cache. Every
    /// caller receives its own DTO; query completeness is preserved verbatim.
    pub fn inspect(&self, element: ElementId) -> Result<ElementInspector> {
        let started = std::env::var_os("AGENTIQUE_VIEW_PROFILE")
            .is_some_and(|value| value == "1")
            .then(Instant::now);
        let (hit, result) = self.inspectors.read(element, || {
            Ok(agq_modeling_view::inspect(self.bound.revision(), element)?)
        });
        if let Some(started) = started {
            let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
            let binding = self.binding();
            let record = serde_json::json!({
                "format": "agentique-studio-inspector-cache/1",
                "operation": "revision_reader_inspect",
                "project": binding.project,
                "revision": binding.revision,
                "element": element,
                "cache": if hit { "hit" } else { "miss" },
                "outcome": if result.is_ok() { "ok" } else { "error" },
                "elapsed_ms": elapsed_ms,
                "timing_contract": "Reader call wall time including lookup and DTO clone; a miss includes modeling-view computation. Excludes queue wait and JSON emission. Not a query-phase, GPU or input-latency measurement."
            });
            let _ = writeln!(std::io::stderr().lock(), "{record}");
        }
        result
    }

    /// Explain actual semantic evidence from the retained immutable model.
    pub fn explain(&self, element: ElementId) -> Result<ExplanationProjection> {
        Ok(agq_modeling_view::explain(self.bound.revision(), element)?)
    }

    /// Read the retained revision's authored source; never consult a mutable file.
    pub fn source(&self, element: ElementId) -> Result<SourceProjection> {
        source_projection(self.binding(), &self.bound, element)
    }
}

impl StudioPlatform {
    /// Mint a read-only worker capability after resolving through the ordinary
    /// authenticated service. The native host requests this after loading a view,
    /// so resolution can reuse that revision without sharing the mutation worker.
    pub fn revision_reader(&self, binding: RevisionBinding) -> Result<StudioRevisionReader> {
        authorize_reader(&self.policy, || self.bound(binding))
    }
}

fn authorize_reader(
    policy: &AgentPolicy,
    resolve: impl FnOnce() -> Result<BoundRevision>,
) -> Result<StudioRevisionReader> {
    policy.require(Authority::Read)?;
    let bound = resolve()?;
    let inspectors = InspectorCache::new(RevisionBinding {
        project: bound.manifest().project_id,
        revision: bound.manifest().revision_id,
    });
    Ok(StudioRevisionReader { bound, inspectors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_authority_is_checked_before_revision_resolution() {
        let denied = AgentPolicy {
            actor: "proposal-only".into(),
            permissions: std::collections::BTreeSet::from([Authority::Propose]),
        };
        assert!(matches!(
            authorize_reader(&denied, || panic!("resolution without read authority")),
            Err(PlatformError::Agent(
                agq_modeling_agent::AgentError::Denied(Authority::Read)
            ))
        ));
        let read_only = AgentPolicy {
            actor: "read-only".into(),
            permissions: std::collections::BTreeSet::from([Authority::Read]),
        };
        // Read alone reaches ordinary resolution; failure cannot mint a reader.
        assert!(matches!(
            authorize_reader(&read_only, || Err(PlatformError::Invalid("unavailable revision".into()))),
            Err(PlatformError::Invalid(reason)) if reason == "unavailable revision"
        ));
    }

    #[test]
    fn immutable_reader_can_cross_and_be_retained_by_the_read_worker() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<StudioRevisionReader>();
    }
}
