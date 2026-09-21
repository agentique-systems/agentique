//! Synthetic dense evidence graph; independent of any language or library corpus.
use agq_kernel::{
    DerivationKey, ElementId, MetaclassId, MetamodelId, OutputKey, RuleId, Snapshot,
    derived::DerivationBuilder,
    metamodel::{MetaclassDescriptor, MetamodelDescriptor, MetamodelRegistry, Version},
    provenance::{DeclaredOrigin, Dependency, Explanation, ExplanationPool, FactKey, Origin},
};
use std::{collections::BTreeSet, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const INPUTS: usize = 2_048;
    const OUTPUTS: usize = 8_192;
    let metamodel = MetamodelId::from_u128(1);
    let class = MetaclassId::from_u128(1);
    let subject = ElementId::from_u128(1);
    let rule = RuleId::from_u128(1);
    let registry = MetamodelRegistry::new(
        [MetamodelDescriptor {
            id: metamodel,
            name: "Non-normative dense evidence fixture".into(),
            version: Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
            uri: "urn:agentique:test:dense-evidence:1".into(),
        }],
        [MetaclassDescriptor {
            id: class,
            name: "Node".into(),
            package: vec![],
            metamodel,
            direct_supertypes: BTreeSet::new(),
            is_abstract: false,
        }],
        [],
    )?;
    let base = Snapshot::new(Arc::new(registry));
    let mut change = base.change_set();
    change.create(subject, class, DeclaredOrigin::Authored { source: None });
    let base = base.apply(&change)?;
    let key = |n: usize| DerivationKey {
        rule,
        subject,
        output: OutputKey::from_u128(n as u128),
    };
    let mut pool = ExplanationPool::default();
    let leaf = pool.intern(Explanation {
        rule,
        dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(subject))]),
    });
    let mut builder = DerivationBuilder::new(base.clone());
    for n in 0..INPUTS {
        builder.element_with_explanation(key(n), class, [], leaf.clone());
    }
    let dense = pool.intern(Explanation {
        rule,
        dependencies: (0..INPUTS)
            .map(|n| Dependency::Derived(FactKey::Element(key(n).element_id())))
            .chain([Dependency::Declared(FactKey::Element(subject))])
            .collect(),
    });
    for n in INPUTS..INPUTS + OUTPUTS {
        builder.element_with_explanation(key(n), class, [], dense.clone());
    }
    assert!(
        Arc::strong_count(&dense) > OUTPUTS,
        "queued facts must share evidence"
    );
    println!(
        "queued {} facts, {} derived dependency edges, two shared proofs",
        INPUTS + OUTPUTS,
        INPUTS * OUTPUTS
    );
    let overlay = builder.build()?;
    for n in 0..INPUTS + OUTPUTS {
        let record = overlay
            .model()
            .element(key(n).element_id())
            .expect("deterministic identity");
        let Origin::Derived(proof) = record.origin() else {
            panic!("derived provenance")
        };
        assert!(Arc::ptr_eq(proof, if n < INPUTS { &leaf } else { &dense }));
        assert!(std::ptr::eq(
            proof.as_ref(),
            overlay.explain(FactKey::Element(record.id())).unwrap()
        ));
    }
    assert_eq!(base.model().elements().count(), 1);
    assert_eq!(overlay.facts().count(), INPUTS + OUTPUTS);
    println!(
        "PASS: immutable source, deterministic identities, shared queued/stored proofs, complete acyclic dependencies"
    );
    Ok(())
}
