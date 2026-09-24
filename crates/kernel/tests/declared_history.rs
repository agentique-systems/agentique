mod common;
use agq_kernel::{provenance::*, value::*, *};
use common::*;
use std::collections::BTreeSet;

fn candidate(base: &Snapshot, source: DeclaredOrigin) -> ConstructionView {
    let mut changes = base.change_set();
    changes.create(ENGINE, TYPE, source);
    base.preview(&changes).unwrap()
}

#[test]
fn omission_is_distinct_from_explicit_retirement_without_a_strict_graph() {
    let base = Snapshot::new(registry());
    let history = DeclaredConstructionHistory::from_snapshot(&base);
    let (history, first) = history.reconcile(candidate(&base, authored())).unwrap();
    let (omitted, empty) = history
        .reconcile(base.preview(&base.change_set()).unwrap())
        .unwrap();
    assert!(empty.model().element(ENGINE).is_none());
    let strict_empty = empty.revalidate_declared().unwrap();
    let mut recreate = strict_empty.change_set();
    recreate.create(ENGINE, TYPE, authored());
    assert!(matches!(
        strict_empty.apply(&recreate),
        Err(ModelError::ReusedIdentity(_))
    ));
    // The source ledger proves continuation across an omitted document.
    let (_, repaired) = omitted.reconcile(candidate(&base, authored())).unwrap();
    assert!(
        repaired
            .revalidate_declared()
            .unwrap()
            .model()
            .element(ENGINE)
            .is_some()
    );
    let retired = omitted
        .retire(&DeclaredIdentitySet {
            elements: BTreeSet::from([ENGINE]),
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        retired.reconcile(candidate(&base, authored())),
        Err(ModelError::ReusedIdentity(_))
    ));
    assert!(first.model().element(ENGINE).is_some());
    assert!(
        history.reconcile(candidate(&base, authored())).is_ok(),
        "old history is immutable"
    );
}

#[test]
fn identity_checkpoint_retains_omitted_and_retired_reservations() {
    let base = Snapshot::new(registry());
    let history = DeclaredConstructionHistory::from_snapshot(&base);
    let (history, _) = history.reconcile(candidate(&base, authored())).unwrap();
    let checkpoint = history.identity_checkpoint();
    let restored = DeclaredConstructionHistory::restore_identities(&base, &checkpoint).unwrap();
    assert_eq!(restored.identity_checkpoint(), checkpoint);
    restored.reconcile(candidate(&base, authored())).unwrap();
    let retired = history
        .retire(&DeclaredIdentitySet {
            elements: BTreeSet::from([ENGINE]),
            ..Default::default()
        })
        .unwrap();
    let restored =
        DeclaredConstructionHistory::restore_identities(&base, &retired.identity_checkpoint())
            .unwrap();
    assert!(matches!(
        restored.reconcile(candidate(&base, authored())),
        Err(ModelError::ReusedIdentity(ENGINE))
    ));
    let mut duplicate = checkpoint.clone();
    duplicate.elements.push(checkpoint.elements[0].clone());
    assert!(DeclaredConstructionHistory::restore_identities(&base, &duplicate).is_err());
}

#[test]
fn same_source_node_can_advance_revision_but_a_different_node_cannot_reuse_identity() {
    let base = Snapshot::new(registry());
    let source = |node, revision: u64| DeclaredOrigin::Authored {
        source: Some(SourceOrigin {
            document: DocumentId::from_u128(501),
            revision: SourceRevisionId::from_u128(revision.into()),
            range: ByteRange::new(revision, revision + 3).unwrap(),
            syntax_node: Some(SyntaxNodeId::from_u128(node)),
        }),
    };
    let (history, _) = DeclaredConstructionHistory::from_snapshot(&base)
        .reconcile(candidate(&base, source(10, 1)))
        .unwrap();
    assert!(history.reconcile(candidate(&base, source(10, 2))).is_ok());
    assert!(matches!(
        history.reconcile(candidate(&base, source(11, 2))),
        Err(ModelError::ConstructionHistory(_))
    ));
}

#[test]
fn declared_promotion_rechecks_required_values_and_preserves_history() {
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
    let history = DeclaredConstructionHistory::from_snapshot(&base);
    let (history, incomplete) = history.reconcile(base.preview(&changes).unwrap()).unwrap();
    assert!(matches!(
        incomplete.revalidate_declared(),
        Err(ModelError::Multiplicity { .. })
    ));
    changes.set(
        SPECIALIZES,
        GENERAL,
        SlotValue::Scalar(Value::Reference(ENGINE)),
        authored(),
    );
    let (_, complete) = history.reconcile(base.preview(&changes).unwrap()).unwrap();
    let revision = complete.revision();
    let strict = complete.revalidate_declared().unwrap();
    assert_eq!(strict.revision(), revision);
    let mut remove = strict.change_set();
    remove.remove(SPECIALIZES);
    let removed = strict.apply(&remove).unwrap();
    let history = DeclaredConstructionHistory::from_snapshot(&removed);
    assert!(matches!(
        history.reconcile(base.preview(&changes).unwrap()),
        Err(ModelError::ReusedIdentity(_))
    ));
}

#[test]
fn fresh_candidate_retired_ids_and_record_kinds_are_preserved() {
    let base = Snapshot::new(registry());
    let mut changes = base.change_set();
    changes.create(ENGINE, TYPE, authored()).remove(ENGINE);
    let (history, _) = DeclaredConstructionHistory::from_snapshot(&base)
        .reconcile(base.preview(&changes).unwrap())
        .unwrap();
    assert!(matches!(
        history.reconcile(candidate(&base, authored())),
        Err(ModelError::ReusedIdentity(_))
    ));
    let (history, _) = DeclaredConstructionHistory::from_snapshot(&base)
        .reconcile(candidate(&base, authored()))
        .unwrap();
    let mut changed_kind = base.change_set();
    changed_kind.create(ENGINE, FEATURE, authored());
    assert!(matches!(
        history.reconcile(base.preview(&changed_kind).unwrap()),
        Err(ModelError::ConstructionHistory(_))
    ));
}
