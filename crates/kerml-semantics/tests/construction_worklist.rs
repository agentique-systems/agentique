//! Unpublished frontiers retain deficits without replaying sealed dependencies.
include!("common/namespace_fixture.rs");
use agq_kernel::derived::{DerivationBuilder, DerivationError};
use std::cell::RefCell;

fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
        ..Default::default()
    }
}

struct IncompleteSpecialization {
    visited: RefCell<BTreeSet<ElementId>>,
}
impl PublicationProducerExtension for IncompleteSpecialization {
    fn applies(&self, model: &ModelView, class: MetaclassId) -> bool {
        model.registry().is_subtype(class, c::CLASS).unwrap()
    }
    fn contribute<'m>(
        &self,
        queries: &KerMlQueries<'m>,
        subject: ElementId,
        _: ResultStructureStratum,
        plan: &mut ResultStructurePlan<'m>,
    ) -> Result<(), DerivationError> {
        self.visited.borrow_mut().insert(subject);
        let supers = queries.supertypes(subject);
        let mut evidence = queries.canonical_fact_evidence(FactKey::Element(subject));
        evidence.merge_evidence(supers).unwrap();
        plan.add_derived_element(
            DerivationKey {
                rule: RuleId::from_u128(700),
                subject,
                output: OutputKey::from_u128(701),
            },
            c::SPECIALIZATION,
            BTreeMap::from([(
                p::SPECIALIZATION_SPECIFIC,
                SlotValue::Scalar(Value::Reference(subject)),
            )]),
            Some(subject),
            &evidence,
        )?;
        Ok(())
    }
}

#[test]
fn changing_construction_obligations_remain_incomplete_and_dependency_is_never_scheduled() {
    let base = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(options().baseline_profile).unwrap(),
    ));
    let changes = base.change_set();
    let mut dependency = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    dependency.create(900, c::CLASS);
    let dependency = Arc::new(DerivationBuilder::new(dependency.finish()).build().unwrap());
    let base = Snapshot::with_immutable_dependency(dependency.clone());
    let changes = base.change_set();
    let mut fixture = Fixture {
        base,
        changes,
        owned: BTreeMap::new(),
    };
    fixture.create(1, c::CLASS);
    let candidate = Arc::new(fixture.construction());
    assert!(candidate.obligations().is_empty());
    let mut graphs = Vec::new();
    for (order, strategy) in [
        (
            PublicationWorklistOrder::Fifo,
            PublicationClosureStrategy::Worklist,
        ),
        (
            PublicationWorklistOrder::Lifo,
            PublicationClosureStrategy::Worklist,
        ),
        (
            PublicationWorklistOrder::Partitioned,
            PublicationClosureStrategy::Worklist,
        ),
        (
            PublicationWorklistOrder::Fifo,
            PublicationClosureStrategy::ReferenceFullScan,
        ),
    ] {
        let extension = IncompleteSpecialization {
            visited: RefCell::default(),
        };
        let result = close_construction_structure_with_extension(
            &candidate,
            PublicationClosureOptions {
                order,
                strategy,
                batch_size: 1,
                ..Default::default()
            },
            |overlay| {
                SemanticContext::for_construction_overlay(overlay, options(), BTreeSet::new())
                    .map_err(PublicationOverlayError::Context)
            },
            &extension,
            |_, _, _, _| {},
            |_| {},
        )
        .unwrap();
        assert!(result.converged);
        assert_eq!(result.completeness, Completeness::Incomplete);
        assert!(result.counters.fixed_point_rounds >= 2);
        assert_eq!(result.counters.declared_subjects, 1);
        assert_eq!(*extension.visited.borrow(), BTreeSet::from([id(1)]));
        assert_eq!(result.overlay.obligations().len(), 1);
        assert_eq!(
            result.overlay.obligations()[0].property,
            p::SPECIALIZATION_GENERAL
        );
        assert_eq!(
            result.overlay.model().element(id(900)),
            dependency.model().element(id(900))
        );
        let context =
            SemanticContext::for_construction_overlay(&result.overlay, options(), BTreeSet::new())
                .unwrap();
        assert_eq!(
            context.id().derivation_phase,
            DerivationPhase::PartialDerivationOverlay
        );
        let answer = KerMlQueries::new(context).supertypes(id(1));
        assert_eq!(answer.completeness, Completeness::Incomplete);
        graphs.push(
            result
                .overlay
                .model()
                .elements()
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    assert!(graphs.windows(2).all(|pair| pair[0] == pair[1]));
}
