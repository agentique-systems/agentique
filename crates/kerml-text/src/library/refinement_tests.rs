use super::*;
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::{KerMlQueries, QualifiedName, SemanticContext, SemanticOptions};
use agq_kernel::{
    AssociationOccurrenceId, ChangeSet, DocumentId, MetaclassId, Snapshot, SourceRevisionId,
    metamodel::ValueKind,
    provenance::{ByteRange, DeclaredOrigin, SourceOrigin},
    value::SlotValue,
};
use std::sync::Arc;

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}

struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    owned: BTreeMap<ElementId, Vec<Value>>,
    references: Vec<PendingLibraryReference>,
}
impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V8).unwrap(),
        ));
        Self {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
            references: vec![],
        }
    }
    fn create(&mut self, n: u128, class: MetaclassId) {
        self.changes.create(id(n), class, origin());
        for property in self
            .base
            .model()
            .registry()
            .effective_properties(class)
            .unwrap()
        {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let registry = self.base.model().registry();
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(n.to_string()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                other => panic!("unexpected fixture primitive {other:?}"),
            };
            self.changes
                .set(id(n), property.id, SlotValue::Scalar(value), origin());
        }
    }
    fn value(&mut self, n: u128, property: PropertyId, value: Value) {
        let registry = self.base.model().registry();
        if registry.supports_slot_storage(property).unwrap() {
            self.changes
                .set(id(n), property, SlotValue::Scalar(value), origin());
        } else {
            let descriptor = registry.property(property).unwrap();
            let Value::Reference(target) = value else {
                panic!("reference occurrence");
            };
            self.changes.link(
                AssociationOccurrenceId::from_u128(n * 1_000_000 + property.as_u128() % 1_000_000),
                descriptor.association.unwrap(),
                BTreeMap::from([
                    (property, target),
                    (*descriptor.opposite_ends.first().unwrap(), id(n)),
                ]),
                BTreeMap::new(),
                origin(),
            );
        }
    }
    fn own(&mut self, owner: u128, relationship: u128) {
        self.owned
            .entry(id(owner))
            .or_default()
            .push(Value::Reference(id(relationship)));
    }
    fn member(
        &mut self,
        owner: u128,
        membership: u128,
        target: u128,
        class: MetaclassId,
        name: &str,
    ) {
        self.member_with_kind(owner, membership, target, class, name, c::OWNING_MEMBERSHIP);
    }
    fn member_with_kind(
        &mut self,
        owner: u128,
        membership: u128,
        target: u128,
        class: MetaclassId,
        name: &str,
        kind: MetaclassId,
    ) {
        self.create(target, class);
        self.value(target, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        self.create(membership, kind);
        self.own(owner, membership);
        self.changes.set(
            id(membership),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(target))]),
            origin(),
        );
    }
    fn pending(
        &mut self,
        resolved: &Endpoints,
        relationship: u128,
        property: PropertyId,
        expected: MetaclassId,
        name: &str,
    ) {
        self.references.push(PendingLibraryReference {
            relationship: id(relationship),
            property,
            expected,
            name: QualifiedName {
                absolute: false,
                segments: name.split("::").map(str::to_owned).collect(),
            },
            membership_target: false,
            executable_expression: false,
            origin: SourceOrigin {
                document: DocumentId::from_u128(1),
                revision: SourceRevisionId::from_u128(2),
                range: ByteRange::new(0, 1).unwrap(),
                syntax_node: None,
            },
        });
        if let Some(&target) = resolved.get(&(id(relationship), property)) {
            self.value(relationship, property, Value::Reference(target));
        }
    }
    fn draft(mut self) -> LibraryDraft {
        for (owner, relationships) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(relationships),
                origin(),
            );
        }
        LibraryDraft {
            candidate: self.base.preview(&self.changes).unwrap(),
            profile: BaselineProfile::OPERATIONAL_V8,
            source_map: BTreeMap::new(),
            roots: vec![id(1), id(6)],
            references: self.references,
            superseded_references: vec![],
        }
    }
}

// Three dependent frontiers: namespace import -> alias -> typing. The separate
// root contains a stable reference whose result must remain reusable throughout.
fn reconstruction(resolved: &Endpoints) -> Result<LibraryDraft, LibraryLoadError> {
    let mut f = Fixture::new();
    f.create(1, c::NAMESPACE);
    f.member(1, 3, 2, c::NAMESPACE, "Library");
    f.member(1, 5, 4, c::NAMESPACE, "Consumer");
    f.create(6, c::NAMESPACE);
    f.member(2, 9, 8, c::CLASS, "Target");
    f.create(10, c::NAMESPACE_IMPORT);
    f.own(4, 10);
    f.pending(
        resolved,
        10,
        p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE,
        c::NAMESPACE,
        "Library",
    );
    f.create(11, c::MEMBERSHIP);
    f.own(4, 11);
    f.value(11, p::MEMBERSHIP_MEMBER_NAME, Value::String("Alias".into()));
    f.pending(
        resolved,
        11,
        p::MEMBERSHIP_MEMBER_ELEMENT,
        c::ELEMENT,
        "Target",
    );
    f.member(4, 13, 12, c::FEATURE, "Use");
    f.create(14, c::FEATURE_TYPING);
    f.own(12, 14);
    f.value(
        14,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(12)),
    );
    f.pending(resolved, 14, p::FEATURE_TYPING_TYPE, c::TYPE, "Alias");
    f.member(6, 16, 15, c::CLASS, "Known");
    f.member(6, 18, 17, c::FEATURE, "Stable");
    f.create(19, c::FEATURE_TYPING);
    f.own(17, 19);
    f.value(
        19,
        p::FEATURE_TYPING_TYPED_FEATURE,
        Value::Reference(id(17)),
    );
    f.pending(resolved, 19, p::FEATURE_TYPING_TYPE, c::TYPE, "Known");
    f.member_with_kind(8, 21, 20, c::FEATURE, "original", c::FEATURE_MEMBERSHIP);
    f.member_with_kind(12, 23, 22, c::FEATURE, "renamed", c::FEATURE_MEMBERSHIP);
    f.create(24, c::REDEFINITION);
    f.own(22, 24);
    f.value(
        24,
        p::REDEFINITION_REDEFINING_FEATURE,
        Value::Reference(id(22)),
    );
    f.pending(
        resolved,
        24,
        p::REDEFINITION_REDEFINED_FEATURE,
        c::FEATURE,
        "original",
    );
    f.member(4, 27, 26, c::FEATURE, "Subset");
    f.create(25, c::SUBSETTING);
    f.own(26, 25);
    f.value(
        25,
        p::SUBSETTING_SUBSETTING_FEATURE,
        Value::Reference(id(26)),
    );
    f.pending(
        resolved,
        25,
        p::SUBSETTING_SUBSETTED_FEATURE,
        c::FEATURE,
        "Library::Target::original",
    );
    Ok(f.draft())
}
fn context(draft: &LibraryDraft) -> SemanticContext<'_> {
    SemanticContext::for_construction(
        &draft.candidate,
        SemanticOptions {
            baseline_profile: draft.profile,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
}
fn queries(draft: &LibraryDraft) -> Result<KerMlStatusQueries<'_>, LibraryLoadError> {
    Ok(KerMlStatusQueries::new(context(draft)))
}
fn run(strategy: ReferenceRefinementStrategy) -> (LibraryDraft, Vec<ReferenceRefinementRound>) {
    let mut reports = vec![];
    let draft = refine(reconstruction, queries, strategy, |round| {
        reports.push(round.clone())
    })
    .unwrap();
    (draft, reports)
}

#[test]
fn dependency_refinement_matches_independent_full_scan_and_reuses_unrelated_queries() {
    let (cached, cached_rounds) = run(ReferenceRefinementStrategy::DependencyDriven);
    let (scanned, scanned_rounds) = run(ReferenceRefinementStrategy::ReferenceFullScan);
    assert_eq!(
        cached.candidate.model().elements().collect::<Vec<_>>(),
        scanned.candidate.model().elements().collect::<Vec<_>>()
    );
    assert_eq!(
        cached
            .candidate
            .model()
            .association_occurrences()
            .collect::<Vec<_>>(),
        scanned
            .candidate
            .model()
            .association_occurrences()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        cached.candidate.obligations(),
        scanned.candidate.obligations()
    );
    assert_eq!(cached.references, scanned.references);
    assert_eq!(cached.source_map, scanned.source_map);
    assert_eq!(cached_rounds.len(), scanned_rounds.len());
    assert!(
        cached_rounds.len() >= 4,
        "import -> alias -> typing requires successive immutable reconstructions: {cached_rounds:?}"
    );
    eprintln!(
        "reference fixture: rounds={}, dependency evaluations={}, reused={}, full-scan evaluations={}",
        cached_rounds.len(),
        cached_rounds
            .iter()
            .map(|r| r.references_evaluated)
            .sum::<usize>(),
        cached_rounds
            .iter()
            .map(|r| r.references_reused)
            .sum::<usize>(),
        scanned_rounds
            .iter()
            .map(|r| r.references_evaluated)
            .sum::<usize>()
    );
    assert_eq!(cached_rounds.last().unwrap().selected_endpoints, 6);
    assert!(
        cached_rounds
            .iter()
            .map(|r| r.references_reused)
            .sum::<usize>()
            > 0
    );
    assert!(
        cached_rounds
            .iter()
            .map(|r| r.references_evaluated)
            .sum::<usize>()
            < scanned_rounds
                .iter()
                .map(|r| r.references_evaluated)
                .sum::<usize>()
    );
    for (cached, scanned) in cached_rounds.iter().zip(&scanned_rounds) {
        assert_eq!(cached.selected_endpoints, scanned.selected_endpoints);
        assert_eq!(
            cached.references_evaluated + cached.references_reused,
            cached.references_considered
        );
        assert_eq!(scanned.references_reused, 0);
    }
    // The ordinary evidence-bearing query results remain equal after refinement.
    let a = KerMlQueries::new(context(&cached));
    let b = KerMlQueries::new(context(&scanned));
    assert_eq!(a.context().model_digest, b.context().model_digest);
    for reference in &cached.references {
        let a = a.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        let b = b.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        assert_eq!(a.value, b.value);
        assert_eq!(a.completeness, b.completeness);
        assert_eq!(a.positive_dependencies, b.positive_dependencies);
        assert_eq!(a.explanations, b.explanations);
        assert_eq!(a.canonical_dependencies, b.canonical_dependencies);
        assert_eq!(a.search_dependencies, b.search_dependencies);
        assert_eq!(a.diagnostics, b.diagnostics);
    }
}

#[test]
fn changed_population_includes_old_and_new_occurrence_endpoints_and_owners() {
    fn candidate(target: Option<u128>) -> LibraryDraft {
        let mut f = Fixture::new();
        f.create(1, c::NAMESPACE);
        f.member(1, 3, 2, c::FEATURE, "a");
        f.member(1, 5, 4, c::FEATURE, "b");
        f.member(1, 7, 6, c::FEATURE, "c");
        f.create(8, c::CROSS_SUBSETTING);
        f.own(2, 8);
        if let Some(target) = target {
            f.value(
                8,
                p::CROSS_SUBSETTING_CROSSED_FEATURE,
                Value::Reference(id(target)),
            );
        }
        f.draft()
    }
    let before = candidate(Some(4));
    let after = candidate(Some(6));
    assert_eq!(
        before.candidate.model().association_occurrences().count(),
        1
    );
    let affected = changed_population(&before.candidate, &after.candidate);
    for participant in [1, 2, 3, 4, 5, 6, 7, 8] {
        assert!(affected.contains(&id(participant)), "missing {participant}");
    }
    let removed = candidate(None);
    let affected = changed_population(&before.candidate, &removed.candidate);
    assert!(affected.contains(&id(4)));
    assert!(affected.contains(&id(8)));
    assert!(changed_population(&before.candidate, &candidate(Some(4)).candidate).is_empty());
}

#[test]
fn negative_lookup_reads_invalidate_when_import_population_becomes_available() {
    let empty = reconstruction(&Endpoints::new()).unwrap();
    let q = queries(&empty).unwrap();
    let reference = &empty.references[1];
    let miss = q.lookup_relationship_target_with_reads(
        reference.relationship,
        reference.property,
        &reference.name,
    );
    assert!(miss.outcome.value.is_empty());
    let available = reconstruction(&Endpoints::from([(
        (id(10), p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE),
        id(2),
    )]))
    .unwrap();
    let mut affected = changed_population(&empty.candidate, &available.candidate);
    let q2 = queries(&available).unwrap();
    let contract_changed = context_changes(q.context(), q2.context(), &mut affected);
    assert!(!contract_changed);
    assert!(miss.reads.affected_by(&affected, contract_changed));
    let found =
        q2.lookup_relationship_target(reference.relationship, reference.property, &reference.name);
    assert_eq!(found.value.len(), 1);
    assert_eq!(found.value[0].element, id(8));
}

#[test]
fn pending_scope_changes_are_local_but_semantic_contract_changes_invalidate_all() {
    let draft = reconstruction(&Endpoints::new()).unwrap();
    let q = queries(&draft).unwrap();
    let reference = &draft.references[1];
    let answer = q.lookup_relationship_target_with_reads(
        reference.relationship,
        reference.property,
        &reference.name,
    );
    let mut current = q.context().clone();
    current.pending_namespace_scopes.insert(id(4));
    let mut affected = BTreeSet::new();
    assert!(!context_changes(q.context(), &current, &mut affected));
    assert_eq!(affected, BTreeSet::from([id(4)]));
    assert!(answer.reads.affected_by(&affected, false));
    let mut current = q.context().clone();
    current.options.exclude_implied = !current.options.exclude_implied;
    assert!(context_changes(q.context(), &current, &mut BTreeSet::new()));
    assert!(answer.reads.affected_by(&BTreeSet::new(), true));
}
