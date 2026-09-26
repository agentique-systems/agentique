//! Cache mechanics only: these DTOs cannot mint a semantic model or BoundRevision.
use super::*;
use agq_kernel::ElementId;
use agq_modeling_agent::{AgentError, AgentPolicy, Authority};
use agq_modeling_repository::{ProjectId, ProjectRevisionId};
use agq_modeling_view::{
    FeatureCounts, GraphScope, RelationshipFamily, ViewEdge, ViewGroup, ViewKind, ViewMetadata,
    ViewNode, ViewOrigin,
};
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

fn projection(binding: RevisionBinding, view: &ViewDefinition) -> ViewProjection {
    let id = ElementId::from_u128(1);
    ViewProjection {
        revision_id: binding.revision,
        view: view.clone(),
        nodes: vec![ViewNode {
            id,
            revision_id: binding.revision,
            semantic_kind: "Unit-only kind".into(),
            name: "Unit-only completed DTO".into(),
            qualified_name: None,
            owner: None,
            origin: ViewOrigin::Authored,
            source_available: false,
            features: vec![],
            counts: FeatureCounts::default(),
            badges: vec!["Unit-only".into()],
        }],
        edges: vec![ViewEdge {
            id: "unit-edge".into(),
            relationship_id: None,
            revision_id: binding.revision,
            family: RelationshipFamily::Reference,
            semantic_kind: "Unit-only edge".into(),
            source: id,
            target: id,
            origin: ViewOrigin::Derived,
            rule_id: None,
            label: "Unit-only cycle".into(),
            directed: true,
            order: 0,
        }],
        groups: vec![ViewGroup {
            element_id: id,
            children: vec![id],
        }],
        metadata: ViewMetadata {
            suggested_focus: Some(id),
            scope: format!("Unit-only project {}", binding.project),
            producer_completeness: "Unit-only Incomplete".into(),
            local_element_count: 1,
            omitted_standard_endpoints: 2,
            warnings: vec!["Unit-only missing evidence".into()],
        },
    }
}

fn read(
    cache: &ProjectionCache,
    binding: RevisionBinding,
    view: &ViewDefinition,
) -> (CacheAccess, Result<ViewProjection>) {
    cache.read(
        binding,
        view,
        || Ok((binding, ())),
        |()| Ok(projection(binding, view)),
    )
}

#[test]
fn complete_dto_is_exact_while_resolution_runs_on_every_hit() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let view = ViewDefinition::architecture();
    let expected = projection(binding, &view);
    let resolutions = AtomicUsize::new(0);
    let computations = AtomicUsize::new(0);
    for expected_access in [CacheAccess::Miss, CacheAccess::Hit, CacheAccess::Hit] {
        let (access, result) = cache.read(
            binding,
            &view,
            || {
                resolutions.fetch_add(1, Ordering::Relaxed);
                Ok((binding, ()))
            },
            |()| {
                assert!(
                    cache.entries.try_lock().is_ok(),
                    "computation must not hold cache lock"
                );
                computations.fetch_add(1, Ordering::Relaxed);
                Ok(expected.clone())
            },
        );
        assert_eq!(access, expected_access);
        let mut result = result.unwrap();
        assert_eq!(result, expected);
        assert_eq!(
            serde_json::to_vec(&result).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        result.metadata.warnings.clear();
        result.metadata.producer_completeness = "caller changed only its copy".into();
        result.nodes[0].name = "caller-owned clone".into();
        result.edges.clear();
        result.groups.clear();
    }
    assert_eq!(resolutions.load(Ordering::Relaxed), 3);
    assert_eq!(computations.load(Ordering::Relaxed), 1);
}

#[test]
fn every_view_field_and_ordered_filter_list_participates_in_exact_key() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let base = ViewDefinition {
        hidden_elements: vec![ElementId::from_u128(8), ElementId::from_u128(9)],
        ..ViewDefinition::architecture()
    };
    assert_eq!(read(&cache, binding, &base).0, CacheAccess::Miss);
    for field in 0..11 {
        let mut view = base.clone();
        match field {
            0 => view.version += 1,
            1 => view.name.push_str(" renamed"),
            2 => view.kind = ViewKind::SemanticGraph,
            3 => view.graph_scope = GraphScope::DependencyNeighborhood,
            4 => view.focus = Some(ElementId::from_u128(2)),
            5 => view.depth += 1,
            6 => view.relationship_families.reverse(),
            7 => {
                view.relationship_families.pop();
            }
            8 => view.include_standard_library = true,
            9 => view.hidden_elements.reverse(),
            _ => view.hidden_elements.push(ElementId::from_u128(10)),
        }
        let (access, result) = read(&cache, binding, &view);
        assert_eq!(access, CacheAccess::Miss, "field {field}");
        assert_eq!(result.unwrap().view, view);
        assert_eq!(read(&cache, binding, &view).0, CacheAccess::Hit);
    }
    assert_eq!(read(&cache, binding, &base).0, CacheAccess::Hit);
    assert_eq!(cache.entries.lock().unwrap().0.len(), 12);
}

#[test]
fn project_revision_and_platform_instance_boundaries_do_not_share_entries() {
    let cache = ProjectionCache::default();
    let first = binding();
    let view = ViewDefinition::architecture();
    let bindings = [
        first,
        RevisionBinding {
            project: ProjectId::new(),
            ..first
        },
        RevisionBinding {
            revision: ProjectRevisionId::new(),
            ..first
        },
    ];
    for binding in bindings {
        let (access, result) = read(&cache, binding, &view);
        assert_eq!(access, CacheAccess::Miss);
        assert_eq!(result.unwrap(), projection(binding, &view));
    }
    for binding in bindings {
        assert_eq!(read(&cache, binding, &view).0, CacheAccess::Hit);
    }
    assert_eq!(
        read(&ProjectionCache::default(), first, &view).0,
        CacheAccess::Miss
    );
}

#[test]
fn denied_read_or_failed_or_wrong_binding_resolution_cannot_use_cached_dto() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let view = ViewDefinition::architecture();
    assert!(read(&cache, binding, &view).1.is_ok());
    let denied = AgentPolicy {
        actor: "unit proposal-only".into(),
        permissions: [Authority::Propose].into(),
    };
    let (access, result) = cache.read(
        binding,
        &view,
        || {
            denied.require(Authority::Read)?;
            Ok((binding, ()))
        },
        |()| panic!("denied projection computed"),
    );
    assert_eq!(access, CacheAccess::NotConsulted);
    assert!(matches!(
        result,
        Err(PlatformError::Agent(AgentError::Denied(Authority::Read)))
    ));
    let (access, result) = cache.read::<()>(
        binding,
        &view,
        || Err(PlatformError::Invalid("revision unavailable".into())),
        |()| panic!("failed binding computed"),
    );
    assert_eq!(access, CacheAccess::NotConsulted);
    assert!(
        matches!(result, Err(PlatformError::Invalid(reason)) if reason == "revision unavailable")
    );
    for actual in [
        RevisionBinding {
            project: ProjectId::new(),
            ..binding
        },
        RevisionBinding {
            revision: ProjectRevisionId::new(),
            ..binding
        },
    ] {
        let (access, result) = cache.read(
            binding,
            &view,
            || Ok((actual, ())),
            |()| panic!("foreign binding computed"),
        );
        assert_eq!(access, CacheAccess::NotConsulted);
        assert!(result.is_err());
    }
    assert_eq!(read(&cache, binding, &view).0, CacheAccess::Hit);
}

#[test]
fn returned_outer_nested_revisions_and_full_view_must_match_before_retention() {
    for wrong in 0..4 {
        let cache = ProjectionCache::default();
        let binding = binding();
        let view = ViewDefinition::architecture();
        let mut value = projection(binding, &view);
        match wrong {
            0 => value.revision_id = ProjectRevisionId::new(),
            1 => value.nodes[0].revision_id = ProjectRevisionId::new(),
            2 => value.edges[0].revision_id = ProjectRevisionId::new(),
            _ => value.view.depth += 1,
        }
        let (access, result) = cache.read(binding, &view, || Ok((binding, ())), |()| Ok(value));
        assert_eq!(access, CacheAccess::Miss);
        assert!(result.is_err());
        assert!(cache.entries.lock().unwrap().0.is_empty());
        assert_eq!(read(&cache, binding, &view).0, CacheAccess::Miss);
    }
}

#[test]
fn failed_computation_is_never_negative_cached() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let view = ViewDefinition::architecture();
    for _ in 0..2 {
        let (access, result) = cache.read(
            binding,
            &view,
            || Ok((binding, ())),
            |()| Err(PlatformError::Invalid("query failed".into())),
        );
        assert_eq!(access, CacheAccess::Miss);
        assert!(matches!(result, Err(PlatformError::Invalid(reason)) if reason == "query failed"));
        assert!(cache.entries.lock().unwrap().0.is_empty());
    }
    assert_eq!(read(&cache, binding, &view).0, CacheAccess::Miss);
}

#[test]
fn sixteen_entries_evict_least_recently_used_definition_without_stale_reuse() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let view = |index| ViewDefinition {
        name: format!("Unit view {index}"),
        ..ViewDefinition::architecture()
    };
    for index in 0..CAPACITY {
        assert_eq!(read(&cache, binding, &view(index)).0, CacheAccess::Miss);
    }
    assert_eq!(read(&cache, binding, &view(0)).0, CacheAccess::Hit);
    assert_eq!(read(&cache, binding, &view(CAPACITY)).0, CacheAccess::Miss);
    assert_eq!(read(&cache, binding, &view(0)).0, CacheAccess::Hit);
    assert_eq!(read(&cache, binding, &view(1)).0, CacheAccess::Miss);
    assert_eq!(cache.entries.lock().unwrap().0.len(), CAPACITY);
}

#[test]
fn concurrent_completion_equality_is_required_and_first_result_is_retained() {
    for conflicting in [false, true] {
        let cache = Arc::new(ProjectionCache::default());
        let barrier = Arc::new(Barrier::new(2));
        let binding = binding();
        let view = ViewDefinition::architecture();
        let workers: Vec<_> = (0..2)
            .map(|index| {
                let cache = cache.clone();
                let barrier = barrier.clone();
                let view = view.clone();
                std::thread::spawn(move || {
                    cache.read(
                        binding,
                        &view,
                        || Ok((binding, ())),
                        |()| {
                            let mut value = projection(binding, &view);
                            if conflicting {
                                value.metadata.warnings.push(format!("unit result {index}"));
                            }
                            barrier.wait(); // Both computations must run outside the mutex.
                            Ok(value)
                        },
                    )
                })
            })
            .collect();
        let results: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert!(
            results
                .iter()
                .all(|(access, _)| *access == CacheAccess::Miss)
        );
        assert_eq!(
            results.iter().filter(|(_, result)| result.is_err()).count(),
            usize::from(conflicting)
        );
        let retained = results
            .into_iter()
            .find_map(|(_, result)| result.ok())
            .unwrap();
        let (access, later) = read(&cache, binding, &view);
        assert_eq!(access, CacheAccess::Hit);
        assert_eq!(later.unwrap(), retained);
        assert_eq!(cache.entries.lock().unwrap().0.len(), 1);
    }
}

#[test]
fn poisoned_cache_bypasses_retention_but_keeps_binding_and_result_checks() {
    let cache = ProjectionCache::default();
    let binding = binding();
    let view = ViewDefinition::architecture();
    let _ = std::panic::catch_unwind(|| {
        let _guard = cache.entries.lock().unwrap();
        panic!("unit-only cache poison");
    });
    for _ in 0..2 {
        let (access, result) = read(&cache, binding, &view);
        assert_eq!(access, CacheAccess::Miss);
        assert_eq!(result.unwrap(), projection(binding, &view));
    }
    let mut invalid = projection(binding, &view);
    invalid.revision_id = ProjectRevisionId::new();
    assert!(
        cache
            .read(binding, &view, || Ok((binding, ())), |()| Ok(invalid))
            .1
            .is_err()
    );
}
