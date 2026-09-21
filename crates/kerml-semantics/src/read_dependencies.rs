//! Shared bounded-read contract for producer scheduling and cached query outcomes.
//! Element-level keys conservatively cover property and association navigation;
//! context identities are checked independently of graph population changes.
use crate::*;
use agq_kernel::{
    ElementId, ModelView,
    derived::StructuralSearch,
    provenance::{Dependency, FactKey},
};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InvalidationKey {
    Element(ElementId),
    Incoming(ElementId),
    Global,
}

fn search_keys(search: &SearchDependency) -> Vec<InvalidationKey> {
    use InvalidationKey as K;
    use SearchDependency as S;
    match search {
        S::Element(id) => vec![K::Element(*id)],
        S::PropertySet { element, .. } => vec![K::Element(*element)],
        S::Incoming { target } => vec![K::Incoming(*target)],
        S::NamespaceMembers { namespace } | S::ImportSet { namespace } => {
            vec![K::Element(*namespace)]
        }
        S::ImportedNamespace { import, namespace } => {
            vec![K::Element(*import), K::Element(*namespace)]
        }
        S::RedefinitionScope {
            relationship,
            namespace,
            ..
        } => vec![K::Element(*relationship), K::Element(*namespace)],
        S::Kernel(StructuralSearch::Element(id)) => vec![K::Element(*id)],
        S::Kernel(
            StructuralSearch::Property { element, .. }
            | StructuralSearch::Association { element, .. },
        ) => vec![K::Element(*element)],
        S::Kernel(StructuralSearch::Incoming(id)) => vec![K::Incoming(*id)],
        S::Kernel(StructuralSearch::Model) | S::Instances { .. } => vec![K::Global],
        // These identities are immutable during this additive session. Binding
        // role reads are local provenance reads, not global role-population scans.
        S::Kernel(StructuralSearch::DescriptorGraph)
        | S::StandardLibraries
        | S::ProjectRoots { .. }
        | S::FormalConstraintTarget(_)
        | S::ValidationRule(_)
        | S::ImpliedBindingRole(_) => vec![],
    }
}
/// Translate language-level reads into persistent kernel computation searches.
/// Context identities (profile, bindings, available roots) are fixed for closure.
pub(crate) fn structural_searches<T>(answer: &QueryResult<T>) -> BTreeSet<StructuralSearch> {
    let mut result = BTreeSet::new();
    for search in &answer.search_dependencies {
        if let SearchDependency::Kernel(search) = search {
            result.insert(search.clone());
        } else if let SearchDependency::PropertySet { element, property } = search {
            result.insert(StructuralSearch::Property {
                element: *element,
                property: *property,
            });
        } else {
            result.extend(search_keys(search).into_iter().map(|key| match key {
                InvalidationKey::Element(id) => StructuralSearch::Element(id),
                InvalidationKey::Incoming(id) => StructuralSearch::Incoming(id),
                InvalidationKey::Global => StructuralSearch::Model,
            }));
        }
    }
    result
}
pub(crate) fn query_read_keys<T>(
    answer: &QueryResult<T>,
    model: &ModelView,
) -> BTreeSet<InvalidationKey> {
    let mut keys: BTreeSet<_> = answer
        .search_dependencies
        .iter()
        .flat_map(search_keys)
        .collect();
    // Immediate canonical dependencies suffice: kernel computation searches
    // propagate the bounded negative reads of any derived facts queried.
    for dependency in &answer.canonical_dependencies {
        let (Dependency::Declared(fact) | Dependency::Derived(fact)) = dependency;
        match fact {
            FactKey::Element(id) | FactKey::Property { element: id, .. } => {
                keys.insert(InvalidationKey::Element(*id));
            }
            FactKey::AssociationOccurrence(id) => {
                if let Some(occurrence) = model.association_occurrence(*id) {
                    keys.extend(
                        occurrence
                            .ends()
                            .values()
                            .copied()
                            .map(InvalidationKey::Element),
                    );
                }
            }
        }
    }
    keys
}
/// Bounded positive and negative reads for selective outcome invalidation.
/// This is a read set, not an expanded explanation or a publication certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryReadSet {
    pub(crate) canonical_dependencies: BTreeSet<Dependency>,
    pub(crate) search_dependencies: BTreeSet<SearchDependency>,
    pub(crate) keys: BTreeSet<InvalidationKey>,
}

/// Compact outcome-cache invalidation keys, without retained query evidence.
///
/// The affected-element protocol already treats element changes and incoming
/// navigation changes identically. One sorted identity population therefore
/// preserves the full read set's invalidation decisions. Public evidence-bearing
/// queries and `QueryReadSet` remain available when explanations are needed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryInvalidationSet {
    elements: Box<[ElementId]>,
    entire_model: bool,
}
impl QueryInvalidationSet {
    /// Distinct bounded positive/negative read subjects in identity order.
    pub fn bounded_elements(&self) -> &[ElementId] {
        &self.elements
    }
    pub fn reads_entire_model(&self) -> bool {
        self.entire_model
    }
    /// Uses exactly the same affected-element/context protocol as `QueryReadSet`.
    pub fn affected_by(
        &self,
        affected_elements: &BTreeSet<ElementId>,
        context_contract_changed: bool,
    ) -> bool {
        if context_contract_changed {
            return true;
        }
        if affected_elements.is_empty() {
            return false;
        }
        if self.entire_model {
            return true;
        }
        // Probe the smaller population; late refinement frontiers commonly
        // change only a handful of records in a much larger namespace read set.
        if affected_elements.len() < self.elements.len() {
            affected_elements
                .iter()
                .any(|id| self.elements.binary_search(id).is_ok())
        } else {
            self.elements
                .iter()
                .any(|id| affected_elements.contains(id))
        }
    }
}

impl QueryReadSet {
    pub fn canonical_dependencies(&self) -> &BTreeSet<Dependency> {
        &self.canonical_dependencies
    }
    pub fn search_dependencies(&self) -> &BTreeSet<SearchDependency> {
        &self.search_dependencies
    }
    /// Bounded read subjects; callers collecting a set deduplicate identities
    /// read through both their records and their incoming navigation.
    pub fn bounded_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.keys.iter().filter_map(|key| match key {
            InvalidationKey::Element(id) | InvalidationKey::Incoming(id) => Some(*id),
            InvalidationKey::Global => None,
        })
    }
    pub fn reads_entire_model(&self) -> bool {
        self.keys.contains(&InvalidationKey::Global)
    }
    /// Discard the proof/search payload after deriving a compact invalidation
    /// contract. This conversion never drops a positive or negative read key.
    pub fn into_invalidation(self) -> QueryInvalidationSet {
        drop(self.canonical_dependencies);
        drop(self.search_dependencies);
        let mut entire_model = false;
        let mut elements = Vec::with_capacity(self.keys.len());
        for key in self.keys {
            match key {
                InvalidationKey::Element(id) | InvalidationKey::Incoming(id) => elements.push(id),
                InvalidationKey::Global => entire_model = true,
            }
        }
        elements.sort_unstable();
        elements.dedup();
        QueryInvalidationSet {
            elements: elements.into_boxed_slice(),
            entire_model,
        }
    }
    /// `affected_elements` includes changed records/owners and BOTH old and new
    /// reference and occurrence endpoints, including inverse navigation. Changes
    /// to pending scopes or construction obligations also contribute their subject
    /// IDs. A changed static context contract invalidates every outcome.
    pub fn affected_by(
        &self,
        affected_elements: &BTreeSet<ElementId>,
        context_contract_changed: bool,
    ) -> bool {
        context_contract_changed
            || (!affected_elements.is_empty()
                && (self.keys.contains(&InvalidationKey::Global)
                    || affected_elements.iter().any(|id| {
                        self.keys.contains(&InvalidationKey::Element(*id))
                            || self.keys.contains(&InvalidationKey::Incoming(*id))
                    })))
    }
    /// Compare immutable query-contract inputs across a reconstructed candidate.
    /// Revision/content digests and pending-state populations may change. Callers
    /// account for those graph and pending-state changes through `affected_by`.
    /// The exact formal target/profile/library contract must remain unchanged;
    /// its per-frontier digest and evidence are allowed to rebind.
    pub fn context_compatible(previous: &SemanticContextId, current: &SemanticContextId) -> bool {
        let mut current = current.clone();
        current.revision = previous.revision;
        current.model_digest = previous.model_digest;
        current.library_graph_digest = previous.library_graph_digest;
        current.pending_specialization_scopes = previous.pending_specialization_scopes.clone();
        current.pending_namespace_scopes = previous.pending_namespace_scopes.clone();
        current.construction_obligations = previous.construction_obligations.clone();
        if let (Some(current_targets), Some(previous_targets)) = (
            &current.formal_constraint_targets,
            &previous.formal_constraint_targets,
        ) && current_targets.same_binding_contract(previous_targets)
        {
            current.formal_constraint_targets = previous.formal_constraint_targets.clone();
        }
        &current == previous
    }
}

#[cfg(test)]
#[path = "../tests/unit/invalidation_set.rs"]
mod tests;
