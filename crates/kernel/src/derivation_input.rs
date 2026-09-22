//! Shared validation input; construction views are never promoted to snapshots.
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum DerivationInput {
    Strict(Snapshot),
    Construction(Arc<ConstructionView>),
}
impl DerivationInput {
    pub(crate) fn model(&self) -> &ModelView {
        match self {
            Self::Strict(v) => v.model(),
            Self::Construction(v) => v.model(),
        }
    }
    pub(crate) fn revision(&self) -> RevisionId {
        match self {
            Self::Strict(v) => v.revision(),
            Self::Construction(v) => v.revision(),
        }
    }
    pub(crate) fn immutable_dependency(&self) -> Option<&Arc<crate::derived::DerivedOverlay>> {
        match self {
            Self::Strict(v) => v.immutable_dependency(),
            Self::Construction(v) => v.immutable_dependency(),
        }
    }
    pub(crate) fn strict(&self) -> &Snapshot {
        match self {
            Self::Strict(v) => v,
            Self::Construction(_) => unreachable!("typed strict derivation input"),
        }
    }
    pub(crate) fn construction(&self) -> &Arc<ConstructionView> {
        match self {
            Self::Construction(v) => v,
            Self::Strict(_) => unreachable!("typed construction derivation input"),
        }
    }
    pub(crate) fn has_used(&self, id: ElementId) -> bool {
        match self {
            Self::Strict(v) => v.has_used(id),
            Self::Construction(v) => v.used_ids.contains(&id),
        }
    }
    pub(crate) fn has_used_occurrence(&self, id: AssociationOccurrenceId) -> bool {
        match self {
            Self::Strict(v) => v.has_used_occurrence(id),
            Self::Construction(v) => v.used_links.contains(&id),
        }
    }
    pub(crate) fn has_declared_fact(&self, fact: FactKey) -> bool {
        match self {
            Self::Strict(v) => v.has_declared_fact(fact),
            Self::Construction(v) => v.model().declared_fact_origin(fact).is_some(),
        }
    }
    pub(crate) fn check_dependency_write(&self, fact: FactKey) -> Result<(), ModelError> {
        match self {
            Self::Strict(v) => v.check_dependency_write(fact),
            Self::Construction(v) => v.base.check_dependency_write(fact),
        }
    }
    pub(crate) fn check_dependency_ownership(&self, model: &ModelView) -> Result<(), ModelError> {
        match self {
            Self::Strict(v) => v.check_dependency_ownership(model),
            Self::Construction(v) => v.base.check_dependency_ownership(model),
        }
    }
    pub(crate) fn build_model(
        &self,
        registry: Arc<MetamodelRegistry>,
        records: BTreeMap<ElementId, Arc<ElementRecord>>,
        links: BTreeMap<AssociationOccurrenceId, AssociationOccurrence>,
        navigation: crate::association::Navigation,
    ) -> Result<(ModelView, Vec<ConstructionObligation>), ModelError> {
        let mut validation = match self {
            Self::Strict(_) => Validation::strict(),
            Self::Construction(_) => Validation {
                deficits: Some(BTreeMap::new()),
            },
        };
        let model = ModelView::build_with_validation(
            registry,
            records,
            links,
            navigation,
            &mut validation,
        )?;
        Ok((
            model,
            validation
                .deficits
                .into_iter()
                .flat_map(BTreeMap::into_values)
                .collect(),
        ))
    }
}
