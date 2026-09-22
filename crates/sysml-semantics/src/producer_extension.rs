//! SysML contributions use the existing KerML dependency-driven scheduler.
use crate::{
    StandardSysmlBindings, SysmlBaselineProfile, SysmlProducerResult, plan_sysml_may_time_vary,
    plan_sysml_producers, sysml_producers_apply,
};
use agq_kerml_semantics::{
    KerMlQueries, PublicationProducerExtension, ResultStructurePlan, ResultStructureStratum,
};
use agq_kernel::{ElementId, MetaclassId, ModelView, derived::DerivationError};

/// Frozen language inputs for one combined closure. The scheduler determines
/// local subjects, query-read invalidation and immutable frontier ordering.
pub struct SysmlProducerExtension {
    profile: SysmlBaselineProfile,
    bindings: StandardSysmlBindings,
    roots: Vec<ElementId>,
}
impl SysmlProducerExtension {
    /// Roots include local Systems declarations and accepted KerML namespaces.
    /// Candidate bindings remain candidates; this object confers no acceptance.
    pub fn new(
        profile: SysmlBaselineProfile,
        bindings: StandardSysmlBindings,
        roots: Vec<ElementId>,
    ) -> Self {
        Self {
            profile,
            bindings,
            roots,
        }
    }
}

fn contribute(
    result: SysmlProducerResult,
    plan: &mut ResultStructurePlan<'_>,
) -> Result<(), DerivationError> {
    plan.observe_evidence(result.evidence.clone())?;
    for relationship in result.relationships {
        plan.add_derived_element(
            relationship.key,
            relationship.metaclass,
            relationship.slots(),
            Some(relationship.specific),
            &result.evidence,
        )?;
    }
    for property in result.properties {
        plan.add_derived_property(
            property.subject,
            property.property,
            property.value,
            property.rule,
            &result.evidence,
        )?;
    }
    Ok(())
}
impl PublicationProducerExtension for SysmlProducerExtension {
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        sysml_producers_apply(model, class)
    }
    fn has_stable_properties(&self) -> bool {
        true
    }
    fn contribute<'m>(
        &self,
        queries: &KerMlQueries<'m>,
        subject: ElementId,
        stratum: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), DerivationError> {
        for result in
            plan_sysml_producers(queries, self.profile, &self.bindings, &self.roots, subject)
                .results
        {
            contribute(result, plan)?;
        }
        if stratum != ResultStructureStratum::Structural
            && queries.model().element(subject).is_some_and(|record| {
                queries
                    .model()
                    .registry()
                    .is_subtype(record.metaclass(), agq_sysml::classes::USAGE)
                    .unwrap_or(false)
            })
        {
            contribute(
                plan_sysml_may_time_vary(
                    queries,
                    self.profile,
                    &self.bindings,
                    &self.roots,
                    subject,
                ),
                plan,
            )?;
        }
        Ok(())
    }
}
