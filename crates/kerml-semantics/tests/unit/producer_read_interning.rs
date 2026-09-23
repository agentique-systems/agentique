use super::*;
use crate as agq_kerml_semantics;
include!("../common/result_fixture.rs");

#[test]
fn repeated_population_reads_share_atom_allocations_across_subjects_and_families() {
    let mut f = Fixture::new();
    f.create(1, c::CLASSIFIER);
    for n in 0..64 {
        f.create(100 + n, c::FEATURE);
        member(&mut f, 1, 100 + n, 1000 + n, c::FEATURE_MEMBERSHIP);
    }
    for n in 0..128 {
        f.create(2000 + n, c::FEATURE);
    }
    let snapshot = f.finish();
    let registry = ProducerRegistry::new(
        ["Fixture.A", "Fixture.B", "Fixture.C", "Fixture.D"]
            .into_iter()
            .map(|name| {
                ProducerDescriptor::new(
                    ProducerFamilyId::new(name),
                    [ProducerEffect::Membership],
                    ProducerApplicability::Any,
                )
            }),
    )
    .unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
    );
    let mut table = ProducerEvaluationTable::default();
    for n in 0..128 {
        let subject = id(2000 + n);
        table.pending(subject, snapshot.model(), &registry);
        // Every subject uses the same owned-feature population and the same
        // negative redefinition search, plus its own existence guard. Capture
        // independently for each family, as ordinary producer planning does.
        for descriptor in registry.descriptors() {
            let mut answer = q.direct_features(id(1));
            assert_eq!(answer.value.len(), 64);
            answer.merge(q.owned_relationships_of_type(id(1), c::REDEFINITION));
            answer.merge(q.canonical_fact_evidence(FactKey::Element(subject)));
            table.record_reads(
                &[(
                    subject,
                    descriptor.id,
                    producer_reads(&answer, snapshot.model()),
                )],
                &registry,
            );
        }
    }
    let mut addresses = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut logical = 0;
    for row in table.reads.values() {
        for (_, reads) in row {
            for read in reads.iter() {
                logical += 1;
                addresses.insert(std::ptr::from_ref(read) as usize);
                values.insert(read);
            }
        }
    }
    assert!(values.contains(&ProducerRead::Owned(id(1), c::REDEFINITION)));
    assert!(
        logical > values.len() * 32,
        "fixture must contain substantial repeated reads"
    );
    let storage = read_storage::ReadStorage::observe(
        table
            .reads
            .values()
            .flat_map(|row| row.iter().map(|(_, reads)| reads)),
    );
    let pool_reference_bytes = table.read_pool.len() * std::mem::size_of::<Arc<ProducerRead>>();
    let unshared_atom_bytes = logical * std::mem::size_of::<ProducerRead>();
    eprintln!(
        "producer read storage: rows={} logical_atoms={logical} distinct_values={} unique_stored_atoms={} unshared_atom_bytes={unshared_atom_bytes} shared_storage={storage:?} pool_reference_bytes={pool_reference_bytes}",
        128 * registry.descriptors().len(),
        values.len(),
        addresses.len(),
    );
    assert_eq!(
        addresses.len(),
        values.len(),
        "equal retained atoms must share storage across independently captured rows"
    );
    assert!(storage.bytes() + pool_reference_bytes < unshared_atom_bytes / 4);
    // Representation alone must not alter any issued mask, state or persisted
    // certificate byte. The raw oracle retains exactly the same sorted values.
    let raw_table = ProducerEvaluationTable {
        rows: table.rows.clone(),
        reads: table
            .reads
            .iter()
            .map(|(&subject, row)| {
                (
                    subject,
                    row.iter()
                        .map(|(family, reads)| {
                            (*family, reads.iter().cloned().collect::<Vec<_>>().into())
                        })
                        .collect(),
                )
            })
            .collect(),
        read_pool: ProducerReadPool::default(),
    };
    let context = SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new())
        .unwrap()
        .with_producer_registry_digest(registry.digest())
        .unwrap();
    let issue = |table: &ProducerEvaluationTable| {
        ProducerClosureCertificate::issue(snapshot.model(), context.id(), &registry, table, |_| {
            None
        })
    };
    let compact = issue(&table);
    let raw = issue(&raw_table);
    assert_eq!(compact.receipt_value(), raw.receipt_value());
    assert_eq!(
        compact.semantic_closure_digest(),
        raw.semantic_closure_digest()
    );
    assert!(compact.revalidation_storage_bytes() < raw.revalidation_storage_bytes() / 4);
}

#[test]
fn read_atom_pool_preserves_content_and_releases_only_unreferenced_atoms() {
    let raw: ProducerReads = vec![
        ProducerRead::OwnedExcluding(
            id(1),
            c::MEMBERSHIP,
            vec![c::FEATURE_MEMBERSHIP, c::FEATURE_VALUE].into(),
        ),
        ProducerRead::Global,
    ]
    .into();
    let mut pool = ProducerReadPool::default();
    let shared = pool.intern(&raw);
    assert_eq!(raw, shared);
    assert_eq!(format!("{raw:?}"), format!("{shared:?}"));
    let second = pool.intern(&raw);
    assert!(
        shared
            .iter()
            .zip(second.iter())
            .all(|(a, b)| std::ptr::eq(a, b))
    );
    let storage = read_storage::ReadStorage::observe([&shared, &second]);
    assert_eq!(storage.logical_atoms, 4);
    assert_eq!(storage.unique_atoms, 2);
    assert_eq!(
        storage.exclusion_bytes,
        2 * std::mem::size_of::<MetaclassId>()
    );
    drop(shared);
    pool.prune();
    assert_eq!(pool.len(), 2, "retained rows keep their atoms alive");
    drop(second);
    pool.prune();
    assert_eq!(
        pool.len(),
        0,
        "the pool alone must not retain obsolete payloads"
    );
    assert_eq!(raw.len(), 2, "raw query input remains independent");
}
