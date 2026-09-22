//! A small synthetic publication exercises authored integration without reading the
//! pinned corpus. Only the ordinary publication builder may seal its overlay.
//! This private test facade makes no assertion about accepted source-library bytes.
use super::*;
use crate::{
    ProjectChange, SourceLanguage, SourceProject,
    syntax::{ByteRange, TextEdit},
};
use agq_kerml::{classes as c, properties as p};
use agq_kerml_semantics::{
    ContextError, DerivationPhase, LibraryPin, PublicationFamily, QualifiedName, QueryResult,
    Resolution, SemanticContext, SemanticOptions, StandardLibraryArtifact, StandardRole,
};
use agq_kernel::{
    ChangeSet, LibraryId, MetaclassId, PropertyId,
    metamodel::ValueKind,
    provenance::{DeclaredOrigin, Origin},
    value::SlotValue,
};
use std::{collections::BTreeMap, collections::BTreeSet, sync::OnceLock};

struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    next: u128,
    owned: BTreeMap<ElementId, Vec<Value>>,
    libraries: BTreeMap<ElementId, LibraryId>,
    classes: BTreeMap<ElementId, MetaclassId>,
}

impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        Self {
            changes: base.change_set(),
            base,
            next: 1,
            owned: BTreeMap::new(),
            libraries: BTreeMap::new(),
            classes: BTreeMap::new(),
        }
    }

    fn create(&mut self, class: MetaclassId, library: LibraryId) -> ElementId {
        let id = ElementId::from_u128(self.next);
        self.next += 1;
        self.libraries.insert(id, library);
        self.classes.insert(id, class);
        let origin = DeclaredOrigin::StandardLibrary { library };
        self.changes.create(id, class, origin.clone());
        let registry = self.base.model().registry();
        for property in registry.effective_properties(class).unwrap() {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(String::new()),
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
                other => panic!("unhandled fixture primitive: {other:?}"),
            };
            self.changes
                .set(id, property.id, SlotValue::Scalar(value), origin.clone());
        }
        id
    }

    fn value(&mut self, id: ElementId, property: PropertyId, value: Value) {
        self.changes.set(
            id,
            property,
            SlotValue::Scalar(value),
            DeclaredOrigin::StandardLibrary {
                library: self.libraries[&id],
            },
        );
    }

    fn member(
        &mut self,
        owner: ElementId,
        name: &str,
        class: MetaclassId,
        membership_class: MetaclassId,
    ) -> ElementId {
        let library = self.libraries[&owner];
        let target = self.create(class, library);
        self.value(target, p::ELEMENT_DECLARED_NAME, Value::String(name.into()));
        let membership_class = if membership_class == c::FEATURE_MEMBERSHIP
            && !self
                .base
                .model()
                .registry()
                .is_subtype(self.classes[&owner], c::TYPE)
                .unwrap()
        {
            c::OWNING_MEMBERSHIP
        } else {
            membership_class
        };
        let membership = self.create(membership_class, library);
        self.owned
            .entry(owner)
            .or_default()
            .push(Value::Reference(membership));
        self.changes.set(
            membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(target)]),
            DeclaredOrigin::StandardLibrary { library },
        );
        target
    }

    fn finish(mut self) -> Snapshot {
        for (owner, relationships) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(relationships),
                DeclaredOrigin::StandardLibrary {
                    library: self.libraries[&owner],
                },
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}

pub(super) fn synthetic_publication() -> Arc<CanonicalKermlStandardLibraries> {
    static FIXTURE: OnceLock<Arc<CanonicalKermlStandardLibraries>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let mut fixture = Fixture::new();
            let libraries = LibrarySetIdentity {
                artifacts: StandardLibraryArtifact::ALL
                    .into_iter()
                    .enumerate()
                    .map(|(index, artifact)| (artifact, LibraryId::from_u128(100 + index as u128)))
                    .collect(),
                pins: BTreeSet::from([LibraryPin {
                    name: "private synthetic authored-publication fixture".into(),
                    sha256: [0x42; 32],
                }]),
            };
            let roots_by_artifact: BTreeMap<_, _> = libraries
                .artifacts
                .iter()
                .map(|(&artifact, &library)| (artifact, fixture.create(c::NAMESPACE, library)))
                .collect();
            let mut paths: BTreeMap<(StandardLibraryArtifact, Vec<String>), ElementId> =
                BTreeMap::new();
            let mut result_owners = BTreeSet::new();
            for role in StandardRole::ALL {
                let artifact = role.library_artifact();
                let (segments, expected) = role.specification();
                let mut owner = roots_by_artifact[&artifact];
                for index in 0..segments.len() {
                    let prefix: Vec<_> =
                        segments[..=index].iter().map(|s| (*s).to_owned()).collect();
                    let key = (artifact, prefix);
                    if let Some(&id) = paths.get(&key) {
                        owner = id;
                        continue;
                    }
                    let class = if index + 1 == segments.len() {
                        expected
                    } else {
                        match (role, index) {
                            (
                                StandardRole::OccurrenceSnapshots
                                | StandardRole::OccurrenceStartShot,
                                1,
                            ) => c::CLASS,
                            (StandardRole::ThingsThat, 1) => c::FEATURE,
                            (StandardRole::FeatureChainSourceTarget, 1) => c::FUNCTION,
                            (StandardRole::FeatureChainSourceTarget, 2) => c::FEATURE,
                            _ => c::LIBRARY_PACKAGE,
                        }
                    };
                    let registry = fixture.base.model().registry();
                    let is_feature = registry.is_subtype(class, c::FEATURE).unwrap();
                    let needs_result = registry.is_subtype(class, c::FUNCTION).unwrap()
                        || registry.is_subtype(class, c::EXPRESSION).unwrap();
                    owner = fixture.member(
                        owner,
                        segments[index],
                        class,
                        if is_feature {
                            c::FEATURE_MEMBERSHIP
                        } else {
                            c::OWNING_MEMBERSHIP
                        },
                    );
                    paths.insert(key, owner);
                    if needs_result {
                        result_owners.insert(owner);
                    }
                }
            }
            for owner in result_owners {
                fixture.member(owner, "result", c::FEATURE, c::RETURN_PARAMETER_MEMBERSHIP);
            }
            let occurrence = paths[&(
                StandardLibraryArtifact::Semantic,
                vec!["Occurrences".into(), "Occurrence".into()],
            )];
            let variable = fixture.member(
                occurrence,
                "variableFixture",
                c::FEATURE,
                c::FEATURE_MEMBERSHIP,
            );
            fixture.value(variable, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
            let snapshot = fixture.finish();
            let roots: Vec<_> = roots_by_artifact.values().copied().collect();
            let complete = CanonicalPublicationBuilder::new(&snapshot, &roots, &libraries)
                .build(16, |_| {})
                .expect("synthetic graph must pass real producer and capability closure");
            assert_eq!(
                complete.context().derivation_phase,
                DerivationPhase::CompletePublicationOverlay
            );
            assert_eq!(
                complete.checked_items()[&PublicationFamily::StandardBindings],
                StandardRole::ALL.len()
            );
            assert!(
                complete
                    .overlay()
                    .model()
                    .elements()
                    .any(|r| matches!(r.origin(), Origin::Derived(_)))
            );
            Arc::new(CanonicalKermlStandardLibraries {
                shared_overlay: Arc::new(complete.overlay().clone()),
                snapshot,
                complete,
                source_map: BTreeMap::new(),
                roots,
                mandatory_references: 0,
                source_content_set: "test-only synthetic graph; not a pinned source publication"
                    .into(),
            })
        })
        .clone()
}

fn add(path: &str, source: &str) -> ProjectChange {
    ProjectChange::Add {
        path: path.into(),
        language: SourceLanguage::KerMl,
        source: source.into(),
    }
}

fn lookup(project: &SourceProject, segments: &[&str]) -> ElementId {
    let revision = project.current();
    let answer = revision.queries().lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: segments.iter().map(|s| (*s).into()).collect(),
        },
    );
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{segments:?}: {answer:?}"
    );
    assert_eq!(answer.value.len(), 1, "{segments:?}: {answer:?}");
    answer.value[0].element
}
fn complete_value<T>(answer: QueryResult<T>) -> T {
    assert_eq!(
        answer.completeness,
        Completeness::Complete,
        "{:?}",
        answer.diagnostics
    );
    answer.value
}

#[test]
fn shared_publication_resolves_authored_structure_without_copying_library_records() {
    let publication = synthetic_publication();
    let digest = publication.semantic_digest();
    let mut first = SourceProject::with_standard_libraries(publication.clone()).unwrap();
    let mut second = SourceProject::with_standard_libraries(publication.clone()).unwrap();
    assert!(Arc::ptr_eq(
        first.standard_libraries().unwrap(),
        second.standard_libraries().unwrap()
    ));
    let revision = first
        .apply(
            first.current().revision(),
            [add(
                "first.kerml",
                r#"
        namespace User {
            private import Base::*;
            alias Universe for Anything;
            type Special specializes Universe;
            feature value : Universe subsets things;
            feature Parent { feature original : Universe; private feature hidden; }
            feature Changed subsets Parent { feature renamed redefines original; }
        }
        namespace Shadow {
            private import Base::*;
            type Anything specializes Base::Anything;
            feature picked : Anything;
        }
    "#,
            )],
        )
        .unwrap();
    assert!(
        revision.is_complete_slice(),
        "{:?}; {:?}",
        revision.diagnostics(),
        revision.semantic_diagnostics()
    );
    let anything = publication.bindings().get(StandardRole::Anything);
    let things = publication.bindings().get(StandardRole::Things);
    assert_eq!(lookup(&first, &["User", "Universe"]), anything);
    let special = lookup(&first, &["User", "Special"]);
    let value = lookup(&first, &["User", "value"]);
    let original = lookup(&first, &["User", "Parent", "original"]);
    let changed = lookup(&first, &["User", "Changed"]);
    let renamed = lookup(&first, &["User", "Changed", "renamed"]);
    let local = lookup(&first, &["Shadow", "Anything"]);
    let picked = lookup(&first, &["Shadow", "picked"]);
    let q = revision.queries();
    assert!(complete_value(q.all_specializations(special)).contains(&anything));
    assert!(complete_value(q.feature_types(value)).contains(&anything));
    assert!(complete_value(q.all_specializations(value)).contains(&things));
    assert!(complete_value(q.redefined_features(renamed)).contains(&original));
    assert!(!complete_value(q.effective_features(changed)).contains(&original));
    assert!(
        revision
            .references()
            .iter()
            .any(|r| r.specific == picked && r.resolution.value == Resolution::Resolved(local))
    );
    let hidden = q.lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: ["User", "Parent", "hidden"].map(String::from).to_vec(),
        },
    );
    assert_eq!(hidden.completeness, Completeness::Complete);
    assert!(hidden.value.is_empty());
    let other = second.apply(second.current().revision(), [add("second.kerml",
        "namespace Other { private import Base::*; feature value : Anything subsets things; }")]).unwrap();
    assert!(
        other.is_complete_slice(),
        "{:?}",
        other.semantic_diagnostics()
    );
    assert!(other.document_at("first.kerml").is_none());
    assert!(revision.document_at("second.kerml").is_none());
    for authored in [&revision, &other] {
        let context = authored.queries().context().clone();
        assert_eq!(context.publication_dependency_digest, Some(digest));
        assert_eq!(context.pinned_libraries, publication.library_set().pins);
        for (_, id) in publication.bindings().iter() {
            assert!(std::ptr::eq(
                publication.overlay().model().element(id).unwrap(),
                authored.snapshot().model().element(id).unwrap()
            ));
        }
    }
}

#[test]
fn authored_revisions_and_parallel_readers_preserve_the_dependency_and_its_explanations() {
    let publication = synthetic_publication();
    let mut project = SourceProject::with_standard_libraries(publication.clone()).unwrap();
    let old = project
        .apply(
            project.current().revision(),
            [add(
                "first.kerml",
                "namespace User { private import Base::*; feature value : Anything; }",
            )],
        )
        .unwrap();
    assert!(old.is_complete_slice());
    let value = lookup(&project, &["User", "value"]);
    let before = old.queries().feature_types(value);
    let next = project
        .apply(
            old.revision(),
            [
                ProjectChange::Edit {
                    document: old.document_at("first.kerml").unwrap().id(),
                    edit: TextEdit {
                        range: ByteRange::new(0, 0).unwrap(),
                        replacement: "namespace Edited { feature edited : Base::Anything; }\n"
                            .into(),
                    },
                },
                add(
                    "next.kerml",
                    "namespace More { private import Base::*; feature another : Anything; }",
                ),
            ],
        )
        .unwrap();
    assert!(next.is_complete_slice());
    assert_ne!(old.revision(), next.revision());
    let edited = lookup(&project, &["Edited", "edited"]);
    assert_eq!(
        complete_value(next.queries().feature_types(edited)),
        vec![publication.bindings().get(StandardRole::Anything)]
    );
    assert_eq!(
        old.queries()
            .context()
            .standard_bindings
            .as_ref()
            .unwrap()
            .iter()
            .collect::<Vec<_>>(),
        next.queries()
            .context()
            .standard_bindings
            .as_ref()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        publication.semantic_digest(),
        old.queries()
            .context()
            .publication_dependency_digest
            .unwrap()
    );
    let facts: Vec<_> = publication.overlay().facts().collect();
    assert!(!facts.is_empty());
    for (fact, explanation) in &facts {
        assert_eq!(
            old.snapshot()
                .immutable_dependency()
                .unwrap()
                .explain(*fact)
                .unwrap(),
            *explanation
        );
        assert_eq!(
            next.snapshot()
                .immutable_dependency()
                .unwrap()
                .explain(*fact)
                .unwrap(),
            *explanation
        );
    }
    let derived = publication
        .overlay()
        .model()
        .elements()
        .find(|r| matches!(r.origin(), Origin::Derived(_)))
        .unwrap()
        .id();
    for target in [publication.bindings().get(StandardRole::Anything), derived] {
        let mut mutation = next.snapshot().change_set();
        mutation.set(
            target,
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String("mutated".into())),
            DeclaredOrigin::Authored { source: None },
        );
        assert!(matches!(
            next.snapshot().apply(&mutation),
            Err(agq_kernel::ModelError::ImmutableDependency(_))
        ));
        assert_eq!(
            old.snapshot().model().element(target),
            next.snapshot().model().element(target)
        );
    }
    std::thread::scope(|scope| {
        let readers: Vec<_> = (0..4)
            .map(|_| {
                let old = &old;
                let next = &next;
                let publication = &publication;
                let before = &before;
                scope.spawn(move || {
                    for _ in 0..4 {
                        assert_eq!(&old.queries().feature_types(value), before);
                        assert_eq!(
                            next.queries().context().publication_dependency_digest,
                            Some(publication.semantic_digest())
                        );
                    }
                })
            })
            .collect();
        for reader in readers {
            reader.join().unwrap();
        }
    });
}

#[test]
fn publication_profile_and_dependency_identity_cannot_be_rebound_by_an_authored_project() {
    let publication = synthetic_publication();
    let project = SourceProject::with_standard_libraries(publication.clone()).unwrap();
    assert_eq!(project.baseline_profile(), BaselineProfile::OPERATIONAL_V9);
    assert_eq!(
        project.current().queries().context().baseline_profile_id,
        publication.context().baseline_profile_id
    );
    assert!(matches!(
        SemanticContext::for_overlay(
            publication.overlay(),
            SemanticOptions {
                baseline_profile: BaselineProfile::OPERATIONAL_V7,
                exclude_implied: false,
            },
            publication.library_set().pins.clone()
        ),
        Err(ContextError::CorrectionProfileMismatch(_))
    ));
    let unrelated = SourceProject::with_profile(BaselineProfile::OPERATIONAL_V9).unwrap();
    assert!(matches!(
        publication.complete_overlay().project_context(
            unrelated.current().snapshot(),
            unrelated.current().root(),
            BTreeSet::new(),
            BTreeSet::new(),
        ),
        Err(ContextError::PublicationDependencyMismatch)
    ));
}
