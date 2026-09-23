//! Bounded diagnostic traversal; it never participates in closure decisions.
use super::{ProducerRegistry, SemanticClosureRequirement};
use agq_kernel::{ElementId, ModelView};
use std::collections::{BTreeSet, VecDeque};

pub(super) fn typing_blockers(
    model: &ModelView,
    subjects: &[ElementId],
    initial_masks: &[u8],
    typing_dependents: &[Vec<usize>],
    registry: &ProducerRegistry,
    states: &[u8],
) {
    if std::env::var_os("AGQ_PRODUCER_CAUSAL_TRACE").is_none() {
        return;
    }
    let Ok(filter) = std::env::var("AGQ_PRODUCER_CAUSAL_SUBJECTS") else {
        return;
    };
    let requested: BTreeSet<_> = filter
        .split(',')
        .map(|id| id.trim().to_ascii_lowercase())
        .filter(|id| !id.is_empty())
        .take(32)
        .collect();
    if requested.is_empty() {
        return;
    }
    let mut dependencies = vec![Vec::new(); subjects.len()];
    for (source, dependents) in typing_dependents.iter().enumerate() {
        for &dependent in dependents {
            dependencies[dependent].push(source);
        }
    }
    let typing = SemanticClosureRequirement::EffectiveTyping;
    for (start, subject) in subjects.iter().enumerate() {
        if !requested.contains(&subject.to_string()) {
            continue;
        }
        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::from([(start, vec![*subject])]);
        let mut roots = 0;
        while let Some((index, path)) = queue.pop_front() {
            if initial_masks[index] & typing.bit() != 0 {
                let families: Vec<_> = registry
                    .descriptors
                    .iter()
                    .enumerate()
                    .filter_map(|(family, descriptor)| {
                        let pair = index * registry.descriptors.len() + family;
                        let state = (states[pair / 4] >> ((pair % 4) * 2)) & 3;
                        (matches!(state, 1 | 3)
                            && descriptor
                                .effects
                                .iter()
                                .any(|&effect| typing.requires_in_model(effect, model)))
                        .then_some(descriptor.id.name())
                    })
                    .collect();
                eprintln!(
                    "closure typing blocker subject={subject} root={} class={:?} local_families={families:?} path={path:?}",
                    subjects[index],
                    model
                        .element(subjects[index])
                        .map(|record| record.metaclass()),
                );
                roots += 1;
                if roots == 8 {
                    break;
                }
            }
            for &dependency in &dependencies[index] {
                if visited.len() < 512 && visited.insert(dependency) {
                    let mut next_path = path.clone();
                    next_path.push(subjects[dependency]);
                    queue.push_back((dependency, next_path));
                }
            }
        }
        eprintln!(
            "closure typing traversal subject={subject} roots={roots} visited={} bounded={}",
            visited.len(),
            roots == 8 || visited.len() == 512,
        );
    }
}
