//! Immutable declared identity reservations across incomplete source revisions.
use super::*;

/// Explicitly deleted declarations, supplied by the source identity ledger.
/// Temporary omission from a construction is not deletion.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeclaredIdentitySet {
    pub elements: BTreeSet<ElementId>,
    pub occurrences: BTreeSet<AssociationOccurrenceId>,
}

/// Kernel-owned reservations; this carries no query answers or acceptance.
/// Accepted dependency reservations remain borrowed through the anchor snapshot.
#[derive(Clone, Debug)]
pub struct DeclaredConstructionHistory {
    anchor: Snapshot,
    elements: BTreeMap<ElementId, (MetaclassId, DeclaredOrigin)>,
    occurrences: BTreeMap<AssociationOccurrenceId, (AssociationId, DeclaredOrigin)>,
    retired: DeclaredIdentitySet,
}

fn continues(before: &DeclaredOrigin, after: &DeclaredOrigin) -> bool {
    match (before, after) {
        (
            DeclaredOrigin::Authored { source: Some(a) },
            DeclaredOrigin::Authored { source: Some(b) },
        ) => a.document == b.document && a.syntax_node == b.syntax_node,
        _ => before == after,
    }
}

impl DeclaredConstructionHistory {
    /// Start from an existing declared history without importing caller-created
    /// reservation data. Existing retired identities stay retired.
    pub fn from_snapshot(snapshot: &Snapshot) -> Self {
        let mut history = Self {
            anchor: snapshot.clone(),
            elements: BTreeMap::new(),
            occurrences: BTreeMap::new(),
            retired: DeclaredIdentitySet::default(),
        };
        for record in snapshot.model().elements() {
            if !snapshot.is_dependency_element(record.id())
                && let Origin::Declared(origin) = record.origin()
            {
                history
                    .elements
                    .insert(record.id(), (record.metaclass(), origin.clone()));
            }
        }
        for occurrence in snapshot.model().association_occurrences() {
            if !history.protected_occurrence(occurrence.id())
                && let Some(origin) = occurrence.declared_origin()
            {
                history
                    .occurrences
                    .insert(occurrence.id(), (occurrence.association(), origin.clone()));
            }
        }
        history.retired.elements = snapshot
            .inner
            .used_ids
            .iter()
            .copied()
            .filter(|id| !history.protected_element(*id) && !history.elements.contains_key(id))
            .collect();
        history.retired.occurrences = snapshot
            .inner
            .used_links
            .iter()
            .copied()
            .filter(|id| {
                !history.protected_occurrence(*id) && !history.occurrences.contains_key(id)
            })
            .collect();
        history
    }

    fn protected_element(&self, id: ElementId) -> bool {
        self.anchor
            .immutable_dependency()
            .is_some_and(|dependency| {
                dependency.model().element(id).is_some()
                    || dependency.declared().inner.used_ids.contains(&id)
            })
    }
    fn protected_occurrence(&self, id: AssociationOccurrenceId) -> bool {
        self.anchor
            .immutable_dependency()
            .is_some_and(|dependency| {
                dependency.model().association_occurrence(id).is_some()
                    || dependency.declared().inner.used_links.contains(&id)
            })
    }

    /// Retire identities even when the current source cannot form a graph.
    /// Failure leaves the original history unchanged.
    pub fn retire(&self, deleted: &DeclaredIdentitySet) -> Result<Self, ModelError> {
        for &id in &deleted.elements {
            if self.protected_element(id) {
                return Err(ModelError::ImmutableDependency(FactKey::Element(id)));
            }
        }
        for &id in &deleted.occurrences {
            if self.protected_occurrence(id) {
                return Err(ModelError::ImmutableDependency(
                    FactKey::AssociationOccurrence(id),
                ));
            }
        }
        let mut next = self.clone();
        for &id in &deleted.elements {
            next.elements.remove(&id);
            next.retired.elements.insert(id);
        }
        for &id in &deleted.occurrences {
            next.occurrences.remove(&id);
            next.retired.occurrences.insert(id);
        }
        Ok(next)
    }

    /// Reconcile a freshly built declared candidate with the same source
    /// history. Omitted active identities remain reserved until explicit retire.
    /// Neither a changed record kind nor a retired identity may be reused.
    pub fn reconcile(
        &self,
        mut candidate: ConstructionView,
    ) -> Result<(Self, ConstructionView), ModelError> {
        match (
            self.anchor.immutable_dependency(),
            candidate.immutable_dependency(),
        ) {
            (None, None) => {}
            (Some(a), Some(b)) if Arc::ptr_eq(a, b) => {}
            _ => return Err(ModelError::ConstructionHistory("immutable dependency")),
        }
        self.anchor
            .model()
            .registry
            .require_extension_of(candidate.model().registry())?;
        candidate
            .model()
            .registry()
            .require_extension_of(self.anchor.model().registry())?;
        let mut next = self.clone();
        for record in candidate.model().elements() {
            if candidate.is_dependency_element(record.id()) {
                continue;
            }
            let Origin::Declared(origin) = record.origin() else {
                return Err(ModelError::ConstructionHistory(
                    "local derived record in declared construction",
                ));
            };
            if self.retired.elements.contains(&record.id()) {
                return Err(ModelError::ReusedIdentity(record.id()));
            }
            if let Some((class, prior)) = self.elements.get(&record.id())
                && (*class != record.metaclass() || !continues(prior, origin))
            {
                return Err(ModelError::ConstructionHistory(
                    "element kind or source identity",
                ));
            }
            next.elements
                .insert(record.id(), (record.metaclass(), origin.clone()));
        }
        for occurrence in candidate.model().association_occurrences() {
            if self.protected_occurrence(occurrence.id()) {
                continue;
            }
            let Some(origin) = occurrence.declared_origin() else {
                return Err(ModelError::ConstructionHistory(
                    "local derived occurrence in declared construction",
                ));
            };
            if self.retired.occurrences.contains(&occurrence.id()) {
                return Err(ModelError::InvalidAssociationOccurrence(occurrence.id()));
            }
            if let Some((association, prior)) = self.occurrences.get(&occurrence.id())
                && (*association != occurrence.association() || !continues(prior, origin))
            {
                return Err(ModelError::ConstructionHistory(
                    "occurrence kind or source identity",
                ));
            }
            next.occurrences
                .insert(occurrence.id(), (occurrence.association(), origin.clone()));
        }
        // A fresh candidate can itself create and then delete a declaration.
        // Carry those reservations forward too, without retiring declarations
        // merely omitted while their source identity remains live.
        let removed_elements = candidate
            .used_ids
            .iter()
            .copied()
            .filter(|id| !next.protected_element(*id) && !next.elements.contains_key(id))
            .collect::<Vec<_>>();
        let removed_occurrences = candidate
            .used_links
            .iter()
            .copied()
            .filter(|id| !next.protected_occurrence(*id) && !next.occurrences.contains_key(id))
            .collect::<Vec<_>>();
        next.retired.elements.extend(removed_elements);
        next.retired.occurrences.extend(removed_occurrences);
        candidate.used_ids.extend(next.elements.keys().copied());
        candidate
            .used_ids
            .extend(next.retired.elements.iter().copied());
        candidate
            .used_links
            .extend(next.occurrences.keys().copied());
        candidate
            .used_links
            .extend(next.retired.occurrences.iter().copied());
        Ok((next, candidate))
    }
}

impl ConstructionView {
    /// Rerun strict validation of the exact declared frontier, preserving its
    /// identity reservations. Structural validity conveys no language acceptance.
    pub fn revalidate_declared(self) -> Result<Snapshot, ModelError> {
        let source = self.model;
        let mut model = ModelView::build_with_validation(
            source.registry,
            source.records,
            source.links,
            source.derived_navigation,
            &mut Validation::strict(),
            source.indexes.base.clone(),
        )?;
        self.base.check_dependency_ownership(&model)?;
        model.statuses = source.statuses;
        model.declared_source = source.declared_source;
        model.searches = source.searches;
        model.reference_contributions = source.reference_contributions;
        Ok(Snapshot {
            inner: Arc::new(SnapshotData {
                revision: self.revision,
                model,
                used_ids: self.used_ids,
                used_links: self.used_links,
                dependency: self.dependency,
            }),
        })
    }
}
