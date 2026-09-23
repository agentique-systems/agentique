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
    eprintln!(
        "producer read storage: rows={} logical_atoms={logical} distinct_values={} atom_allocations={} atom_bytes={}",
        128 * registry.descriptors().len(),
        values.len(),
        addresses.len(),
        addresses.len() * std::mem::size_of::<ProducerRead>(),
    );
    assert_eq!(
        addresses.len(),
        values.len(),
        "equal retained atoms must share storage across independently captured rows"
    );
}
