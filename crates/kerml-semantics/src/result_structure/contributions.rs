//! Language extension writes share the canonical frontier and ownership merge.
use super::*;

#[derive(PartialEq, Eq)]
pub(super) struct PropertyContribution {
    pub(super) value: SlotValue,
    explanation: KernelExplanation,
    searches: BTreeSet<StructuralSearch>,
}

pub(super) fn merge_property(
    target: &mut BTreeMap<(ElementId, PropertyId), PropertyContribution>,
    key: (ElementId, PropertyId),
    value: PropertyContribution,
) -> Result<(), DerivationError> {
    if target.get(&key).is_some_and(|existing| existing != &value) {
        return Err(DerivationError::DuplicateFact(FactKey::Property {
            element: key.0,
            property: key.1,
        }));
    }
    target.insert(key, value);
    Ok(())
}

pub(super) fn enqueue_properties<Input>(
    model: &agq_kernel::ModelView,
    properties: BTreeMap<(ElementId, PropertyId), PropertyContribution>,
    builder: &mut DerivationBuilder<Input>,
) -> Result<(), DerivationError> {
    for ((element, property), contribution) in properties {
        let fact = FactKey::Property { element, property };
        if let Some(existing) = model.navigation_slot(element, property) {
            if existing.value() != &contribution.value
                || !matches!(existing.origin(), Origin::Derived(proof)
                    if proof.rule == contribution.explanation.rule)
            {
                return Err(DerivationError::DuplicateFact(fact));
            }
            continue;
        }
        builder.property(
            element,
            property,
            contribution.value,
            contribution.explanation,
        );
        builder.searches(fact, contribution.searches);
    }
    Ok(())
}

impl ResultStructurePlan<'_> {
    /// Retain an extension producer's result and its positive and negative reads,
    /// including when the rule cannot yet propose any canonical facts.
    pub fn observe_evidence(&mut self, evidence: QueryResult<()>) -> Result<(), DerivationError> {
        if evidence.context != self.context {
            return Err(DerivationError::InputContextMismatch);
        }
        self.production.merge(evidence);
        Ok(())
    }

    /// Add an evidenced language contribution to this immutable frontier. An
    /// incomplete premise is retained as pending and never creates a fact.
    /// Stable output keys belong to the language's versioned semantic rules.
    pub fn add_derived_element(
        &mut self,
        key: DerivationKey,
        class: MetaclassId,
        slots: BTreeMap<PropertyId, SlotValue>,
        owner: Option<ElementId>,
        evidence: &QueryResult<()>,
    ) -> Result<Option<ElementId>, DerivationError> {
        self.observe_evidence(evidence.clone())?;
        if evidence.completeness != Completeness::Complete {
            return Ok(None);
        }
        let id = self
            .graph
            .create(key, class, &evidence.canonical_dependencies);
        for (property, mut value) in slots {
            if let SlotValue::Bag(values) = &mut value {
                values.sort();
            }
            self.graph.set_value(id, property, value);
        }
        if let Some(owner) = owner {
            self.graph.own(owner, id);
        }
        let searches = self
            .graph
            .search_pool
            .intern(crate::read_dependencies::structural_searches(evidence));
        self.graph.merge_searches(id, searches);
        self.production.value.push(id);
        Ok(Some(id))
    }

    /// Propose a derived scalar or collection after its semantic antecedents
    /// are stable. Existing facts must agree exactly; declared slots are never
    /// overwritten. Callers must honor any required producer stratification.
    pub fn add_derived_property(
        &mut self,
        subject: ElementId,
        property: PropertyId,
        mut value: SlotValue,
        rule: RuleId,
        evidence: &QueryResult<()>,
    ) -> Result<(), DerivationError> {
        self.observe_evidence(evidence.clone())?;
        if evidence.completeness != Completeness::Complete {
            return Ok(());
        }
        if let SlotValue::Bag(values) = &mut value {
            values.sort();
        }
        let dependencies = self
            .graph
            .producer_dependencies(&evidence.canonical_dependencies);
        contributions::merge_property(
            &mut self.graph.contributed_properties,
            (subject, property),
            PropertyContribution {
                value,
                explanation: KernelExplanation { rule, dependencies },
                searches: crate::read_dependencies::structural_searches(evidence),
            },
        )
    }
}
