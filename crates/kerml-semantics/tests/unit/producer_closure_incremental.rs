use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");
use super::{ClosureCertificateBuilder, ProducerEvaluationTable, ProducerRead};

const FAMILY: ProducerFamilyId = ProducerFamilyId::new("Fixture.IncrementalTyping");

fn registry(effect: ProducerEffect) -> ProducerRegistry {
    let mut descriptor = ProducerDescriptor::new(
        FAMILY,
        [effect],
        ProducerApplicability::Subtypes(vec![c::FEATURE]),
    );
    descriptor.scope = ProducerEffectScope::Subject;
    ProducerRegistry::new([descriptor]).unwrap()
}
fn fixture(classes: &[(u128, MetaclassId)]) -> Snapshot {
    let mut f = Fixture::new();
    for &(subject, class) in classes {
        f.create(subject, class);
    }
    f.finish()
}
fn context<'a>(snapshot: &'a Snapshot, registry: &ProducerRegistry) -> SemanticContext<'a> {
    SemanticContext::for_snapshot(snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap()
}
fn table(snapshot: &Snapshot, registry: &ProducerRegistry) -> ProducerEvaluationTable {
    let mut table = ProducerEvaluationTable::default();
    for record in snapshot.model().elements() {
        table.pending(record.id(), snapshot.model(), registry);
    }
    table
}
fn compare(
    builder: &mut ClosureCertificateBuilder,
    snapshot: &Snapshot,
    registry: &ProducerRegistry,
    table: &ProducerEvaluationTable,
    changed: &[ElementId],
) -> ProducerClosureCertificate {
    let context = context(snapshot, registry);
    let actual = builder.issue(
        snapshot.model(),
        context.id(),
        registry,
        table,
        &changed.iter().copied().collect(),
        |_| None,
    );
    let full =
        ProducerClosureCertificate::issue(snapshot.model(), context.id(), registry, table, |_| {
            None
        });
    actual.assert_exact(&full);
    assert_eq!(
        actual.semantic_closure_digest(),
        full.semantic_closure_digest()
    );
    actual
}

#[test]
fn delta_rows_equal_full_rebuild_and_reuse_unchanged_topology() {
    let snapshot = fixture(&[(1, c::FEATURE), (2, c::FEATURE), (3, c::TYPE)]);
    let registry = registry(ProducerEffect::Typing);
    let mut table = table(&snapshot, &registry);
    let mut builder = ClosureCertificateBuilder::default();
    compare(&mut builder, &snapshot, &registry, &table, &[]);
    let topology = builder.cache.topology_rebuilt;
    let scopes = builder.cache.scopes_rebuilt;
    compare(&mut builder, &snapshot, &registry, &table, &[]);
    assert_eq!(topology, builder.cache.topology_rebuilt);
    assert_eq!(scopes, builder.cache.scopes_rebuilt);
    table
        .record(&[(id(1), FAMILY, Completeness::Complete)], &registry)
        .unwrap();
    table.record_reads(
        &[(id(1), FAMILY, vec![ProducerRead::Identity(id(1))].into())],
        &registry,
    );
    compare(&mut builder, &snapshot, &registry, &table, &[]);
    assert_eq!(topology, builder.cache.topology_rebuilt);
    assert_eq!(scopes + 1, builder.cache.scopes_rebuilt);
}

#[test]
fn disappearing_subject_and_changed_class_remove_obsolete_rows() {
    let original = fixture(&[(1, c::FEATURE), (2, c::FEATURE)]);
    let revised = fixture(&[(1, c::TYPE)]);
    let registry = registry(ProducerEffect::Typing);
    let mut builder = ClosureCertificateBuilder::default();
    let before = compare(
        &mut builder,
        &original,
        &registry,
        &table(&original, &registry),
        &[],
    );
    assert!(!before.is_closed(id(1), SemanticClosureRequirement::EffectiveTyping));
    let after = compare(
        &mut builder,
        &revised,
        &registry,
        &table(&revised, &registry),
        &[id(1), id(2)],
    );
    assert!(after.is_fully_closed(revised.model()));
    assert_eq!(after.evaluation(id(2), 0), None);
    assert!(!builder.cache.scopes.contains_key(&id(2)));
    assert!(!builder.cache.topology.contains_key(&id(2)));
}

#[test]
fn changed_effect_scope_and_provider_evidence_cannot_reuse_old_rows() {
    let snapshot = fixture(&[(1, c::FEATURE), (2, c::FEATURE)]);
    let first = registry(ProducerEffect::Typing);
    let mut builder = ClosureCertificateBuilder::default();
    compare(
        &mut builder,
        &snapshot,
        &first,
        &table(&snapshot, &first),
        &[],
    );
    let mut descriptor = first.descriptors()[0].clone();
    descriptor.scope = ProducerEffectScope::Model;
    descriptor.effects = BTreeSet::from([ProducerEffect::Featuring]);
    let revised = ProducerRegistry::new([descriptor]).unwrap();
    let mut table = table(&snapshot, &revised);
    table
        .record(&[(id(1), FAMILY, Completeness::Complete)], &revised)
        .unwrap();
    table.record_reads(
        &[(
            id(1),
            FAMILY,
            vec![ProducerRead::Source(
                id(2),
                c::TYPE_FEATURING,
                p::TYPE_FEATURING_FEATURE_OF_TYPE,
            )]
            .into(),
        )],
        &revised,
    );
    let blocked = compare(&mut builder, &snapshot, &revised, &table, &[]);
    assert_eq!(
        blocked.evaluation(id(1), 0),
        Some(ProducerEvaluationState::Pending)
    );
    table.pending(id(1), snapshot.model(), &revised);
    table
        .record(&[(id(1), FAMILY, Completeness::Complete)], &revised)
        .unwrap();
    table.record_reads(
        &[(id(1), FAMILY, vec![ProducerRead::Identity(id(2))].into())],
        &revised,
    );
    let closed = compare(&mut builder, &snapshot, &revised, &table, &[]);
    assert_eq!(
        closed.evaluation(id(1), 0),
        Some(ProducerEvaluationState::EvaluatedComplete)
    );
    assert_ne!(blocked.transport_reads, closed.transport_reads);
}

fn chained(target: u128) -> Snapshot {
    let mut f = Fixture::new();
    for subject in [1, 2, 3] {
        f.create(subject, c::FEATURE);
    }
    relation(
        &mut f,
        1,
        target,
        10,
        c::FEATURE_CHAINING,
        p::FEATURE_CHAINING_CHAINING_FEATURE,
    );
    f.finish()
}

#[test]
fn changed_chain_endpoint_replaces_cached_negative_topology() {
    let first = chained(2);
    let next = chained(3);
    let registry = registry(ProducerEffect::Typing);
    let mut builder = ClosureCertificateBuilder::default();
    compare(
        &mut builder,
        &first,
        &registry,
        &table(&first, &registry),
        &[],
    );
    let old_row = builder.cache.topology[&id(1)].clone();
    // The chain owner itself is unchanged; its carrier footprint must invalidate.
    compare(
        &mut builder,
        &next,
        &registry,
        &table(&next, &registry),
        &[id(10), id(2), id(3)],
    );
    assert_ne!(
        old_row.dependencies,
        builder.cache.topology[&id(1)].dependencies
    );
    assert!(
        builder.cache.topology[&id(1)]
            .dependencies
            .contains(&(id(3), id(1), 2))
    );
}

#[test]
fn disappearing_source_provider_removes_all_obsolete_requirement_masks() {
    let snapshot = fixture(&[(1, c::FEATURE), (2, c::FEATURE)]);
    let registry = ProducerRegistry::new([]).unwrap();
    let table = table(&snapshot, &registry);
    let clear = context(&snapshot, &registry);
    let mut pending = clear.id().clone();
    pending.pending_namespace_scopes.insert(id(1));
    let mut builder = ClosureCertificateBuilder::default();
    let before = builder.issue(
        snapshot.model(),
        &pending,
        &registry,
        &table,
        &BTreeSet::new(),
        |_| None,
    );
    before.assert_exact(&ProducerClosureCertificate::issue(
        snapshot.model(),
        &pending,
        &registry,
        &table,
        |_| None,
    ));
    assert!(!before.is_fully_closed(snapshot.model()));
    let after = compare(&mut builder, &snapshot, &registry, &table, &[]);
    assert!(after.is_fully_closed(snapshot.model()));
}
