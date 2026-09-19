mod common;
use agq_kernel::{value::*, *};
use common::*;

#[test]
fn incomplete_candidate_is_inspectable_but_publication_remains_strict() {
    let base = Snapshot::new(registry());
    let mut changes = base.change_set();
    changes
        .create(ENGINE, TYPE, authored())
        .create(SPORTS, TYPE, authored())
        .create(SPECIALIZES, SPECIALIZATION, authored())
        .set(
            SPECIALIZES,
            SPECIFIC,
            SlotValue::Scalar(Value::Reference(SPORTS)),
            authored(),
        );
    let candidate = base.preview(&changes).unwrap();
    assert_eq!(candidate.model().len(), 3);
    assert_eq!(
        candidate.obligations(),
        &[ConstructionObligation {
            element: SPECIALIZES,
            property: GENERAL,
            required: agq_kernel::metamodel::Multiplicity::ONE,
            actual: 0,
        }]
    );
    assert!(matches!(
        base.apply(&changes),
        Err(ModelError::Multiplicity { .. })
    ));
    assert!(base.model().is_empty());
    changes.set(
        SPECIALIZES,
        GENERAL,
        SlotValue::Scalar(Value::Reference(ENGINE)),
        authored(),
    );
    let complete = base.preview(&changes).unwrap();
    assert!(complete.obligations().is_empty());
    assert_ne!(candidate.revision(), complete.revision());
    let snapshot = base.apply(&changes).unwrap();
    assert_eq!(
        snapshot.model().elements().collect::<Vec<_>>(),
        complete.model().elements().collect::<Vec<_>>()
    );
    assert_eq!(candidate.obligations().len(), 1);
}

#[test]
fn preview_rejects_dangling_references_bad_types_and_upper_bounds() {
    let base = Snapshot::new(registry());
    for value in [Value::Reference(ENGINE), Value::Boolean(true)] {
        let mut changes = base.change_set();
        changes.create(SPECIALIZES, SPECIALIZATION, authored()).set(
            SPECIALIZES,
            GENERAL,
            SlotValue::Scalar(value),
            authored(),
        );
        assert!(base.preview(&changes).is_err());
    }
    let mut changes = base.change_set();
    changes
        .create(ENGINE, TYPE, authored())
        .create(SPECIALIZES, SPECIALIZATION, authored())
        .set(
            SPECIALIZES,
            TARGETS,
            SlotValue::Ordered(vec![Value::Reference(ENGINE); 4]),
            authored(),
        );
    assert!(matches!(
        base.preview(&changes),
        Err(ModelError::Multiplicity { actual: 4, .. })
    ));
}
