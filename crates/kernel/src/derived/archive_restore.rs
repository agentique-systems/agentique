//! Structural reconstruction for the neutral graph archive. No language seal.
use super::*;
use crate::archive::ArchiveError;
use crate::metamodel::ValueKind;
use crate::shared_map::SharedMap;
use crate::value::Value;

impl DerivedOverlay {
    pub(crate) fn restore_archive(
        declared: Snapshot,
        model: ModelView,
    ) -> Result<Self, ArchiveError> {
        restore(DerivationInput::Strict(declared), model, vec![]).map(|inner| Self { inner })
    }
}

impl ConstructionOverlay {
    pub(crate) fn restore_archive(
        declared: Arc<crate::ConstructionView>,
        model: ModelView,
        obligations: Vec<crate::ConstructionObligation>,
    ) -> Result<Self, ArchiveError> {
        restore(DerivationInput::Construction(declared), model, obligations)
            .map(|inner| Self { inner })
    }
}

fn restore(
    declared: DerivationInput,
    mut model: ModelView,
    obligations: Vec<crate::ConstructionObligation>,
) -> Result<Arc<OverlayData>, ArchiveError> {
    let invalid =
        || ArchiveError::Invalid("overlay does not monotonically extend its declared snapshot");
    // New local relationship carriers must not take ownership of a protected
    // dependency record, even when the archive never rewrites that record.
    declared.check_dependency_ownership(&model)?;
    // An archive cannot relabel, remove, or replace declared assertions. The
    // only stored-slot replacement admitted by DerivationBuilder is appending
    // references to an ordered collection, retaining its declared prefix.
    for before in declared.model().elements() {
        let after = model.element(before.id()).ok_or_else(invalid)?;
        if before.metaclass() != after.metaclass() || before.origin() != after.origin() {
            return Err(invalid());
        }
        for (property, slot) in before.slots() {
            let next = after.slot(property).ok_or_else(invalid)?;
            if slot != next && !extension(&model, property, Some(slot), next)? {
                return Err(invalid());
            }
        }
    }
    for before in declared.model().association_occurrences() {
        if model.association_occurrence(before.id()) != Some(before) {
            return Err(invalid());
        }
    }
    let mut explanations = declared
        .immutable_dependency()
        .map_or_else(SharedMap::new, |d| d.inner.explanations.fork());
    let mut evidence_pool = declared
        .immutable_dependency()
        .map_or_else(ExplanationPool::default, |d| d.inner.evidence_pool.fork());
    for record in model.elements() {
        let previous = declared.model().element(record.id());
        if previous.is_none()
            && (declared.has_used(record.id()) || !matches!(record.origin(), Origin::Derived(_)))
        {
            return Err(invalid());
        }
        origin(
            FactKey::Element(record.id()),
            record.origin(),
            &mut explanations,
            &mut evidence_pool,
        );
        for (property, slot) in record.slots() {
            let before = previous.and_then(|r| r.slot(property));
            if before != Some(slot) {
                if !matches!(slot.origin(), Origin::Derived(_)) {
                    return Err(invalid());
                }
                // New inferred Elements have ordinary stored properties;
                // existing declared Elements only admit derived properties
                // or monotone ordered-reference extensions.
                if previous.is_some()
                    && !model
                        .registry()
                        .property(property)
                        .map_err(ModelError::from)?
                        .derived
                    && !extension(&model, property, before, slot)?
                {
                    return Err(invalid());
                }
            }
            origin(
                FactKey::Property {
                    element: record.id(),
                    property,
                },
                slot.origin(),
                &mut explanations,
                &mut evidence_pool,
            );
        }
    }
    for occurrence in model.association_occurrences() {
        if declared
            .model()
            .association_occurrence(occurrence.id())
            .is_none()
            && (declared.has_used_occurrence(occurrence.id())
                || !matches!(occurrence.origin(), Origin::Derived(_)))
        {
            return Err(invalid());
        }
        origin(
            FactKey::AssociationOccurrence(occurrence.id()),
            occurrence.origin(),
            &mut explanations,
            &mut evidence_pool,
        );
    }
    for ((element, property), slot) in model.derived_navigation_results() {
        if !matches!(slot.origin(), Origin::Derived(_)) {
            return Err(invalid());
        }
        origin(
            FactKey::Property {
                element: *element,
                property: *property,
            },
            slot.origin(),
            &mut explanations,
            &mut evidence_pool,
        );
    }
    for (&(element, property), failure) in model.statuses.local_iter() {
        let record = model
            .element(element)
            .ok_or(ModelError::UnknownElement(element))?;
        if !model
            .registry()
            .is_applicable_navigation(record.metaclass(), property)
            .map_err(ModelError::from)?
        {
            return Err(ModelError::IllegalProperty {
                element,
                class: record.metaclass(),
                property,
            }
            .into());
        }
        if !model
            .registry()
            .property(property)
            .map_err(ModelError::from)?
            .derived
        {
            return Err(DerivationError::NotDerivedProperty { element, property }.into());
        }
        let fact = FactKey::Property { element, property };
        if model.navigation_slot(element, property).is_some() || explanations.contains_key(&fact) {
            return Err(DerivationError::DuplicateFact(fact).into());
        }
        explanations.insert(fact, evidence_pool.intern(failure.explanation().clone()));
        let (ComputationFailure::Incomplete { searches, .. }
        | ComputationFailure::Invalid { searches, .. }) = failure;
        if !model
            .searches
            .get(&fact)
            .is_some_and(|all| searches.is_subset(all))
        {
            return Err(ArchiveError::Invalid("failure search evidence was lost"));
        }
    }
    let mut checked = HashSet::new();
    for (&fact, proof) in &explanations {
        let unsuccessful = matches!(fact, FactKey::Property { element, property } if model.statuses.contains_key(&(element,property)));
        if !checked.insert((Arc::as_ptr(proof) as usize, unsuccessful)) {
            continue;
        }
        for &dependency in &proof.dependencies {
            let exists = match dependency {
                Dependency::Declared(key) => declared.has_declared_fact(key),
                Dependency::Derived(key) => {
                    if !unsuccessful
                        && matches!(key, FactKey::Property { element, property } if model.statuses.contains_key(&(element,property)))
                    {
                        return Err(DerivationError::IncompleteDependency(key).into());
                    }
                    explanations.contains_key(&key)
                }
            };
            if !exists {
                return Err(DerivationError::MissingDependency { fact, dependency }.into());
            }
        }
    }
    // Selected ordered supports are independently retained evidence. They may
    // narrow an aggregate slot proof, but may never introduce an unsupported
    // dependency or a search absent from that slot/element's complete evidence.
    // Subset validation also inherits the aggregate proof's cycle check below.
    for (&(element, property, _), contribution) in &model.reference_contributions {
        let fact = FactKey::Property { element, property };
        let Some(Origin::Derived(aggregate)) =
            model.navigation_slot(element, property).map(Slot::origin)
        else {
            return Err(ArchiveError::Invalid(
                "ordered contribution has no inferred slot",
            ));
        };
        if !contribution
            .explanation()
            .dependencies
            .is_subset(&aggregate.dependencies)
        {
            return Err(ArchiveError::Invalid(
                "ordered contribution has unsupported proof dependencies",
            ));
        }
        let slot_searches = model.searches.get(&fact);
        let element_searches = model.searches.get(&FactKey::Element(element));
        if contribution.searches().iter().any(|search| {
            !slot_searches.is_some_and(|searches| searches.contains(search))
                && !element_searches.is_some_and(|searches| searches.contains(search))
        }) {
            return Err(ArchiveError::Invalid(
                "ordered contribution has unsupported search evidence",
            ));
        }
    }
    let mut search_pool = declared
        .immutable_dependency()
        .map_or_else(StructuralSearchPool::default, |d| {
            d.inner.search_pool.fork()
        });
    for (fact, searches) in &mut model.searches {
        if !explanations.contains_key(fact) {
            return Err(DerivationError::MissingSearchSubject(*fact).into());
        }
        *searches = search_pool.intern_shared(searches.clone());
    }
    let facts = explanations.keys().copied().collect();
    let cycle = cyclic_explanations(&explanations, &evidence_pool, &facts, false);
    if !cycle.is_empty() {
        return Err(DerivationError::DependencyCycle(cycle).into());
    }
    model.declared_source = Some(declared.clone());
    // Reconstruction work is distinct from producer accounting; language
    // receipt metadata may retain the original deterministic counters.
    let build_metrics = DerivationBuildMetrics {
        full_model_validations: 1,
        proof_sets_interned: evidence_pool.statistics().interned,
        logical_search_sets: model.searches.len(),
        logical_search_entries: model.searches.values().map(|s| s.len()).sum(),
        retained_search_sets: search_pool.statistics().interned,
        retained_search_entries: search_pool.statistics().entries,
        ..Default::default()
    };
    Ok(Arc::new(OverlayData {
        declared,
        model,
        obligations,
        explanations,
        evidence_pool,
        search_pool,
        build_metrics,
        element_reservations: OnceLock::new(),
        occurrence_reservations: OnceLock::new(),
        archive_dependency_digest: OnceLock::new(),
        evidence_archive_dependency_digest: OnceLock::new(),
    }))
}
fn origin(
    fact: FactKey,
    origin: &Origin,
    explanations: &mut SharedMap<FactKey, Arc<Explanation>>,
    pool: &mut ExplanationPool,
) {
    if let Origin::Derived(proof) = origin {
        if explanations.get(&fact) == Some(proof) {
            return;
        }
        explanations.insert(fact, pool.intern_shared(proof.clone()));
    }
}
fn extension(
    model: &ModelView,
    property: PropertyId,
    before: Option<&Slot>,
    after: &Slot,
) -> Result<bool, ModelError> {
    let descriptor = model.registry().property(property)?;
    if descriptor.derived
        || !descriptor.ordered
        || !matches!(after.origin(), Origin::Derived(_))
        || !matches!(
            model.registry().storage_kind(descriptor.value_kind)?,
            ValueKind::Reference(_)
        )
        || !model.registry().supports_slot_storage(property)?
    {
        return Ok(false);
    }
    let SlotValue::Ordered(after) = after.value() else {
        return Ok(false);
    };
    let prefix = match before.map(Slot::value) {
        None => &[][..],
        Some(SlotValue::Ordered(values)) => values.as_slice(),
        _ => return Ok(false),
    };
    let unique: BTreeSet<_> = after.iter().collect();
    Ok(after.starts_with(prefix)
        && unique.len() == after.len()
        && after.iter().all(|v| matches!(v, Value::Reference(_))))
}
