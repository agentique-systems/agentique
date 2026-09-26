//! Experimental, structurally checked reconstruction. No language acceptance.
use super::*;

/// Verification-only reconstruction of a requested subset of old derived facts.
/// Search validity and producer closure remain the language layer's obligation.
pub struct VerificationRetainedFrontier {
    pub overlay: DerivedOverlay,
    pub retained: BTreeSet<FactKey>,
    pub retracted: usize,
}

impl DerivedOverlay {
    /// Rebuild on new declarations with a subset of exact old local facts.
    ///
    /// This experimental kernel operation grants structural validity only. It
    /// cannot certify that a language rule or negative search remains true.
    /// The caller must retract obsolete semantic results before evaluating new
    /// language rules. The immutable dependency and registry must be the exact
    /// same allocations. Declared inputs, identities and ownership are never
    /// replaced by old derived data.
    ///
    /// Changed positive declared support, missing transitive derived support,
    /// failed computations and partially retained created records are retracted.
    /// Every retained proof, search and ordered contribution keeps its original
    /// bytes. The merged graph passes ordinary strict structural validation.
    pub fn verification_retain_on(
        &self,
        declared: Snapshot,
        requested: &BTreeSet<FactKey>,
    ) -> Result<VerificationRetainedFrontier, DerivationError> {
        let old_input = &self.inner.declared;
        let input = DerivationInput::Strict(declared);
        let same_dependency = match (
            old_input.immutable_dependency(),
            input.immutable_dependency(),
        ) {
            (Some(old), Some(new)) => Arc::ptr_eq(old, new),
            (None, None) => true,
            _ => false,
        };
        if !same_dependency || !Arc::ptr_eq(&old_input.model().registry, &input.model().registry) {
            return Err(DerivationError::InputContextMismatch);
        }
        let local: BTreeSet<_> = self
            .inner
            .explanations
            .local_iter()
            .map(|(&fact, _)| fact)
            .collect();
        if !requested.is_subset(&local) {
            return Err(DerivationError::InputContextMismatch);
        }
        let dependency = input.immutable_dependency();
        let mut retained = requested.clone();
        loop {
            let mut remove = BTreeSet::new();
            for &fact in &retained {
                if matches!(fact, FactKey::Property { element, property }
                    if self.model().statuses.contains_key(&(element, property)))
                {
                    remove.insert(fact);
                    continue;
                }
                let proof = &self.inner.explanations[&fact];
                if proof
                    .dependencies
                    .iter()
                    .any(|dependency_read| match dependency_read {
                        Dependency::Declared(key) => {
                            !input.has_declared_fact(*key)
                                || !same_fact(old_input.model(), input.model(), *key)
                        }
                        Dependency::Derived(key) => {
                            !retained.contains(key)
                                && !dependency.is_some_and(|base| base.explain(*key).is_some())
                        }
                    })
                {
                    remove.insert(fact);
                }
            }
            // Created records are retained atomically. Dropping a required slot
            // cannot leave an apparently valid partial semantic output behind.
            for record in self.model().records.local_values() {
                if !matches!(record.origin(), Origin::Derived(_)) {
                    continue;
                }
                let element = FactKey::Element(record.id());
                let slots: Vec<_> = record
                    .slots()
                    .map(|(property, _)| FactKey::Property {
                        element: record.id(),
                        property,
                    })
                    .collect();
                if !retained.contains(&element)
                    || remove.contains(&element)
                    || slots
                        .iter()
                        .any(|fact| !retained.contains(fact) || remove.contains(fact))
                {
                    remove.insert(element);
                    remove.extend(slots);
                }
            }
            let before = retained.len();
            retained.retain(|fact| !remove.contains(fact));
            if retained.len() == before {
                break;
            }
        }
        let mut parts = input.model().derivation_parts();
        let mut explanations =
            dependency.map_or_else(SharedMap::new, |base| base.inner.explanations.fork());
        // These immutable pools retain old allocations, but are not authority:
        // only the explicitly filtered fact maps enter the reconstructed view.
        let mut evidence_pool = self.inner.evidence_pool.clone();
        let search_pool = self.inner.search_pool.clone();
        for record in self.model().records.local_values() {
            if retained.contains(&FactKey::Element(record.id())) {
                if input.has_used(record.id()) || parts.records.contains_key(&record.id()) {
                    return Err(DerivationError::IdentityCollision(record.id()));
                }
                parts.records.insert(record.id(), record.clone());
            } else if let Some(current) = parts.records.get_mut(&record.id()) {
                for (property, slot) in record.slots() {
                    let fact = FactKey::Property {
                        element: record.id(),
                        property,
                    };
                    if retained.contains(&fact) {
                        input.check_dependency_write(fact)?;
                        Arc::make_mut(current).slots.insert(property, slot.clone());
                    }
                }
            }
        }
        for occurrence in self.model().links.local_values() {
            if retained.contains(&FactKey::AssociationOccurrence(occurrence.id())) {
                if input.has_used_occurrence(occurrence.id())
                    || parts.links.contains_key(&occurrence.id())
                {
                    return Err(DerivationError::AssociationIdentityCollision(
                        occurrence.id(),
                    ));
                }
                parts.links.insert(occurrence.id(), occurrence.clone());
            }
        }
        for (&(element, property), slot) in self.model().derived_navigation.local_iter() {
            if retained.contains(&FactKey::Property { element, property }) {
                input.check_dependency_write(FactKey::Property { element, property })?;
                parts
                    .derived_navigation
                    .insert((element, property), slot.clone());
            }
        }
        for &fact in &retained {
            explanations.insert(
                fact,
                evidence_pool.intern_shared(self.inner.explanations[&fact].clone()),
            );
            if let Some(searches) = self.model().searches.get(&fact) {
                parts.searches.insert(fact, searches.clone());
            }
        }
        for (&(element, property, target), contribution) in
            self.model().reference_contributions.local_iter()
        {
            if retained.contains(&FactKey::Property { element, property }) {
                parts
                    .reference_contributions
                    .insert((element, property, target), contribution.clone());
            }
        }
        let (mut model, obligations) = input.build_model(
            input.model().registry.clone(),
            parts.records,
            parts.links,
            parts.derived_navigation,
        )?;
        input.check_dependency_ownership(&model)?;
        model.declared_source = Some(input.clone());
        model.statuses = parts.statuses;
        model.searches = parts.searches;
        model.reference_contributions = parts.reference_contributions;
        // All retained edges existed in the previously checked acyclic proof
        // graph; changed declared leaves were removed above. A subset cannot
        // introduce a proof cycle. Structural validation still checks every
        // resulting endpoint, required bound and protected dependency owner.
        let overlay = DerivedOverlay {
            inner: Arc::new(OverlayData {
                declared: input,
                model,
                obligations,
                explanations,
                evidence_pool,
                search_pool,
                build_metrics: DerivationBuildMetrics {
                    existing_facts_reused: retained.len(),
                    full_model_validations: 1,
                    ..Default::default()
                },
                element_reservations: OnceLock::new(),
                occurrence_reservations: OnceLock::new(),
                archive_dependency_digest: OnceLock::new(),
                evidence_archive_dependency_digest: OnceLock::new(),
            }),
        };
        Ok(VerificationRetainedFrontier {
            retracted: requested.len() - retained.len(),
            retained,
            overlay,
        })
    }

    /// Exact local derived fact keys, excluding the immutable dependency.
    pub fn verification_local_facts(&self) -> BTreeSet<FactKey> {
        self.inner
            .explanations
            .local_iter()
            .map(|(&fact, _)| fact)
            .collect()
    }
}

fn same_fact(old: &ModelView, new: &ModelView, fact: FactKey) -> bool {
    match fact {
        FactKey::Element(id) => old.element(id) == new.element(id),
        FactKey::Property { element, property } => {
            old.navigation_slot(element, property) == new.navigation_slot(element, property)
        }
        FactKey::AssociationOccurrence(id) => {
            old.association_occurrence(id) == new.association_occurrence(id)
        }
    }
}
