//! Effective names without choosing an arbitrary implied-relationship order.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, value::Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Names are established only when every permitted naming source agrees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectiveNames {
    Determinate(BTreeSet<String>),
    /// The semantic checks require these targets but do not specify their mutual
    /// order as inserted owned Redefinitions. No target is selected by storage ID.
    Ambiguous {
        alternatives: BTreeSet<BTreeSet<String>>,
        authority: &'static str,
    },
}

impl KerMlQueries<'_> {
    /// Feature::effectiveName/effectiveShortName and namingFeature (8.3.3.3.4).
    /// Explicit owned Redefinitions use exact ownership order. For implied-only
    /// redefinitions, agreement proves names independently of any insertion order.
    pub fn effective_names(&self, element: ElementId) -> QueryResult<EffectiveNames> {
        if let Some(cached) = self
            .effective_names_cache
            .lock()
            .expect("effective names cache")
            .get(&element)
        {
            return (**cached).clone();
        }
        let result = self.compute_effective_names(element);
        self.effective_names_cache
            .lock()
            .expect("effective names cache")
            .insert(element, std::sync::Arc::new(result.clone()));
        result
    }

    // Namespace lookups repeatedly inspect the same members' names. Cache the
    // bounded naming answer in this immutable evaluator, including incomplete
    // outcomes and all proof/search metadata. A fork starts with an empty memo.
    fn compute_effective_names(&self, element: ElementId) -> QueryResult<EffectiveNames> {
        let mut out = self.result(EffectiveNames::Determinate(BTreeSet::new()));
        let mut graph = BTreeMap::<ElementId, Vec<ElementId>>::new();
        let mut values = BTreeMap::<ElementId, BTreeSet<BTreeSet<String>>>::new();
        let mut queue = VecDeque::from([element]);
        while let Some(id) = queue.pop_front() {
            if graph.contains_key(&id) {
                continue;
            }
            self.fact(&mut out, agq_kernel::provenance::FactKey::Element(id));
            let mut names = BTreeSet::new();
            for property in [p::ELEMENT_DECLARED_NAME, p::ELEMENT_DECLARED_SHORT_NAME] {
                if let Some(Value::String(name)) = self.read_value(&mut out, id, property) {
                    names.insert(name.clone());
                }
            }
            if !names.is_empty() || !self.is(id, c::FEATURE) {
                values.insert(id, BTreeSet::from([names]));
                graph.insert(id, vec![]);
                continue;
            }
            let owned = self.owned_relationships(id);
            let (targets, rule) = if let Some(&r) =
                owned.value.iter().find(|r| self.is(**r, c::REDEFINITION))
            {
                let target = self.read_reference(&mut out, r, p::REDEFINITION_REDEFINED_FEATURE);
                if target.is_none() {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_NAMING_TARGET",
                        r,
                        "The first owned Redefinition has no established target",
                    );
                }
                (target.into_iter().collect(), Rule::OrderedNamingFeature)
            } else {
                let implied = self.implied_redefinitions(id);
                let targets = implied.value.clone();
                out.merge(implied);
                (targets, Rule::ImpliedNamingAgreement)
            };
            out.merge(owned);
            let premises: Vec<_> = out
                .positive_dependencies
                .iter()
                .copied()
                .map(Evidence::Fact)
                .chain(
                    out.search_dependencies
                        .iter()
                        .cloned()
                        .map(Evidence::Search),
                )
                .collect();
            for &target in &targets {
                out.prove(QueryKind::NamingSource, id, target, rule, premises.clone());
            }
            if targets.is_empty() {
                values.insert(id, BTreeSet::from([BTreeSet::new()]));
            }
            queue.extend(targets.iter().copied());
            graph.insert(id, targets);
        }
        while values.len() < graph.len() {
            let ready: Vec<_> = graph
                .iter()
                .filter(|(id, targets)| {
                    !values.contains_key(id) && targets.iter().all(|t| values.contains_key(t))
                })
                .map(|(id, _)| *id)
                .collect();
            if ready.is_empty() {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_NAMING_CYCLE",
                    element,
                    "Cyclic namingFeature dependencies do not establish an effective name",
                );
                return out;
            }
            for id in ready {
                let alternatives = graph[&id]
                    .iter()
                    .flat_map(|t| values[t].iter().cloned())
                    .collect();
                values.insert(id, alternatives);
            }
        }
        let alternatives = values.remove(&element).unwrap_or_default();
        if alternatives.len() > 1 {
            out.problem(Completeness::Incomplete,"KQ_AMBIGUOUS_IMPLIED_NAME",element,
                format!("Required implied redefinitions admit distinct effective names: {alternatives:?}; KerML 1.0 8.4.2 and namingFeature do not prescribe their mutual insertion order"));
            out.value = EffectiveNames::Ambiguous {
                alternatives,
                authority: "KerML 1.0 8.3.3.3.4 namingFeature; 8.4.2; checkFeatureParameter/Result/EndRedefinition",
            };
        } else {
            out.value =
                EffectiveNames::Determinate(alternatives.into_iter().next().unwrap_or_default());
        }
        out
    }
}

#[cfg(test)]
#[path = "../tests/unit/naming_cache.rs"]
mod tests;
