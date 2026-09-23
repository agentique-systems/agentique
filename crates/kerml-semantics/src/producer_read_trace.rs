//! Opt-in bounded read-origin diagnostics; no evidence or scheduler state changes.
use crate::{
    FeaturePopulationKind, ProducerFamilyId, QueryResult, SearchDependency,
    SemanticClosureRequirement,
    producer_closure::{ProducerRead, ProducerReads},
};
use agq_kernel::{ElementId, ModelView, derived::StructuralSearch};
use std::{cell::Cell, sync::OnceLock};

fn requested(subject: ElementId, family: ProducerFamilyId) -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !ENABLED.get_or_init(|| std::env::var_os("AGQ_PRODUCER_READ_TRACE").is_some()) {
        return false;
    }
    let Ok(subjects) = std::env::var("AGQ_PRODUCER_CAUSAL_SUBJECTS") else {
        return false;
    };
    subjects
        .split(',')
        .take(32)
        .any(|id| id.trim().eq_ignore_ascii_case(&subject.to_string()))
        && std::env::var("AGQ_PRODUCER_READ_FAMILIES")
            .ok()
            .is_none_or(|families| {
                families
                    .split(',')
                    .take(32)
                    .any(|name| name.trim() == family.name())
            })
}

fn kernel_bucket(search: &StructuralSearch, model: &ModelView) -> Option<&'static str> {
    match search {
        StructuralSearch::Incoming(_) | StructuralSearch::Association { .. } => Some("Inverse"),
        StructuralSearch::Model => Some("Global"),
        StructuralSearch::ElementIdentity(element) if model.element(*element).is_none() => {
            Some("Global")
        }
        StructuralSearch::OwnedMemberProjection { contract, .. }
            if FeaturePopulationKind::from_contract_id(contract).is_none() =>
        {
            Some("Global")
        }
        StructuralSearch::ProducerClosure { requirement, .. }
            if SemanticClosureRequirement::from_contract_id(requirement).is_none() =>
        {
            Some("Global")
        }
        _ => None,
    }
}

/// Each selected evaluation emits at most sixteen source records, inspecting at
/// most 4096 retained facts per search. Shared metadata is definitely imported;
/// expanded kernel metadata may also be a direct caller search. Matching facts
/// identify its retained provenance without claiming a unique import route.
pub(crate) fn trace<T>(
    subject: ElementId,
    family: ProducerFamilyId,
    answer: &QueryResult<T>,
    model: &ModelView,
    reads: &ProducerReads,
) {
    if !requested(subject, family)
        || !reads
            .iter()
            .any(|read| matches!(read, ProducerRead::Global | ProducerRead::Inverse))
    {
        return;
    }
    let remaining = Cell::new(16);
    let emit = |bucket: &str,
                channel: &str,
                search: &dyn std::fmt::Debug,
                source: &dyn std::fmt::Debug| {
        if remaining.get() > 0 {
            eprintln!(
                "closure broad read subject={subject} family={} status={:?} model={:02x?} bucket={bucket} channel={channel} search={search:?} source={source:?}",
                family.name(),
                answer.completeness,
                &answer.context.model_digest[..8],
            );
            remaining.set(remaining.get() - 1);
        }
    };
    for (channel, search) in answer
        .shared_search_dependencies
        .iter()
        .map(|search| ("imported-shared", search))
        .chain(answer.search_dependencies.iter().filter_map(|search| {
            if let SearchDependency::Kernel(search) = search {
                Some(("expanded-kernel-or-direct", search))
            } else {
                None
            }
        }))
    {
        if remaining.get() == 0 {
            break;
        }
        let Some(bucket) = kernel_bucket(search, model) else {
            continue;
        };
        let mut attributed = false;
        for fact in answer.positive_dependencies.iter().take(4096) {
            if remaining.get() == 0 {
                break;
            }
            if model
                .computation_searches_shared(*fact)
                .is_some_and(|searches| searches.contains(search))
            {
                emit(bucket, channel, search, fact);
                attributed = true;
            }
        }
        for candidate in answer.search_dependencies.iter().take(4096) {
            if remaining.get() == 0 {
                break;
            }
            if let SearchDependency::Kernel(StructuralSearch::OrderedReferenceContribution {
                element,
                property,
                target,
            }) = candidate
                && model
                    .ordered_reference_contribution(*element, *property, *target)
                    .is_some_and(|contribution| contribution.searches().contains(search))
            {
                emit(bucket, channel, search, &(*element, *property, *target));
                attributed = true;
            }
        }
        if !attributed {
            emit(
                bucket,
                channel,
                search,
                &"unattributed-within-retained-fact-bound",
            );
        }
    }
    for search in &answer.search_dependencies {
        if remaining.get() == 0 {
            break;
        }
        let bucket = match search {
            SearchDependency::Incoming { .. } => Some("Inverse"),
            SearchDependency::Instances { .. } => Some("Global"),
            SearchDependency::Element(element) if model.element(*element).is_none() => {
                Some("Global")
            }
            _ => None,
        };
        if let Some(bucket) = bucket {
            emit(bucket, "direct-language", search, &"current-query-search");
        }
    }
    if remaining.get() == 0 {
        eprintln!(
            "closure broad read limit subject={subject} family={} limit=16 retained_fact_scan_limit=4096",
            family.name()
        );
    }
}

pub(crate) fn reads<T>(
    subject: ElementId,
    family: ProducerFamilyId,
    answer: &QueryResult<T>,
    model: &ModelView,
) -> ProducerReads {
    let reads = crate::producer_closure::producer_reads(answer, model);
    trace(subject, family, answer, model, &reads);
    reads
}

pub(crate) fn missing_reads(subject: ElementId, family: ProducerFamilyId) {
    if requested(subject, family) {
        eprintln!(
            "closure broad read subject={subject} family={} bucket=Global channel=missing-read-row source=scheduler-conservative-fallback",
            family.name(),
        );
    }
}
