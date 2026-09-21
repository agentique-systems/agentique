//! Validation of a dependency graph factored through shared immutable proofs.
use crate::model::cyclic_nodes_by;
use crate::provenance::{Explanation, ExplanationPool, FactKey};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// A fact points to its shared proof, and each proof points to its derived
/// dependencies. This factors N outputs sharing M premises into N + M edges,
/// rather than expanding the same M premises N times during cycle validation.
pub(super) fn cyclic_explanations(
    explanations: &BTreeMap<FactKey, Arc<Explanation>>,
    pool: &ExplanationPool,
    changed: &BTreeSet<FactKey>,
    include_previous: bool,
) -> Vec<FactKey> {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Node {
        Fact(FactKey),
        // The least fact identity using a proof is its deterministic local key.
        Proof(FactKey),
    }
    let mut representatives = BTreeMap::new();
    let mut fact_proofs = BTreeMap::new();
    let mut proof_dependencies = BTreeMap::new();
    let facts: Box<dyn Iterator<Item = FactKey> + '_> = if include_previous {
        Box::new(explanations.keys().copied())
    } else {
        Box::new(changed.iter().copied())
    };
    for fact in facts {
        let proof = &explanations[&fact];
        let representative = *representatives
            .entry(Arc::as_ptr(proof) as usize)
            .or_insert_with(|| {
                proof_dependencies.insert(
                    fact,
                    pool.derived_dependencies(proof)
                        .iter()
                        .copied()
                        .filter(|dependency| include_previous || changed.contains(dependency))
                        .collect::<Vec<_>>(),
                );
                fact
            });
        fact_proofs.insert(fact, representative);
    }
    cyclic_nodes_by(changed.iter().copied().map(Node::Fact), |node| {
        let (proof, dependencies): (_, &[FactKey]) = match node {
            Node::Fact(fact) => (Some(Node::Proof(fact_proofs[fact])), &[]),
            Node::Proof(proof) => (None, &proof_dependencies[proof]),
        };
        proof
            .into_iter()
            .chain(dependencies.iter().copied().map(Node::Fact))
    })
    .into_iter()
    .filter_map(|node| match node {
        Node::Fact(fact) => Some(fact),
        Node::Proof(_) => None,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Dependency;
    use crate::{ElementId, RuleId};

    #[test]
    fn shared_proof_graph_matches_unfactored_cycle_oracle_for_all_three_vertex_graphs() {
        let facts: Vec<_> = (0..3)
            .map(|index| FactKey::Element(ElementId::from_u128(index)))
            .collect();
        let changed = facts.iter().copied().collect();
        for mask in 0..(1 << 9) {
            let mut pool = ExplanationPool::default();
            let mut explanations = BTreeMap::new();
            for (source, &fact) in facts.iter().enumerate() {
                let dependencies = facts
                    .iter()
                    .enumerate()
                    .filter_map(|(target, &fact)| {
                        (mask & (1 << (source * 3 + target)) != 0)
                            .then_some(Dependency::Derived(fact))
                    })
                    .collect();
                explanations.insert(
                    fact,
                    pool.intern(Explanation {
                        rule: RuleId::from_u128(1),
                        dependencies,
                    }),
                );
            }
            let expected = cyclic_nodes_by(facts.iter().copied(), |fact| {
                explanations[fact]
                    .dependencies
                    .iter()
                    .map(|dependency| match dependency {
                        Dependency::Derived(fact) => *fact,
                        Dependency::Declared(_) => unreachable!(),
                    })
            });
            assert_eq!(
                cyclic_explanations(&explanations, &pool, &changed, false),
                expected,
                "mask {mask}"
            );
        }
    }
}
