//! Acceptance regressions over the same publication instance as the release gate.
use agq_kerml::{BaselineProfile, properties as p};
use agq_kerml_semantics::{
    Completeness, QualifiedName, Resolution, SemanticContext, SemanticOptions, StandardRole,
};
use agq_kerml_text::{
    ProjectChange, SourceLanguage, SourceProject, library::CanonicalKermlStandardLibraries,
};
use agq_kernel::{
    ElementId,
    provenance::Origin,
    value::{SlotValue, Value},
};
use std::sync::Arc;

fn add(path: &str, source: &str) -> ProjectChange {
    ProjectChange::Add {
        path: path.into(),
        language: SourceLanguage::KerMl,
        source: source.into(),
    }
}
fn lookup(
    project: &SourceProject,
    segments: &[&str],
) -> Result<ElementId, Box<dyn std::error::Error>> {
    let revision = project.current();
    let answer = revision.queries().lookup_path(
        revision.root(),
        &QualifiedName {
            absolute: false,
            segments: segments.iter().map(|s| (*s).into()).collect(),
        },
    );
    if answer.completeness != Completeness::Complete || answer.value.len() != 1 {
        return Err(format!(
            "Authored lookup {segments:?}: {:?}; {:?}",
            answer.completeness, answer.diagnostics
        )
        .into());
    }
    Ok(answer.value[0].element)
}
pub fn verify(
    publication: Arc<CanonicalKermlStandardLibraries>,
) -> Result<(), Box<dyn std::error::Error>> {
    let digest = publication.semantic_digest();
    let mut project = SourceProject::with_standard_libraries(publication.clone())?;
    let mut second = SourceProject::with_standard_libraries(publication.clone())?;
    assert!(Arc::ptr_eq(
        project.standard_libraries().unwrap(),
        second.standard_libraries().unwrap()
    ));
    assert_eq!(project.baseline_profile(), BaselineProfile::OPERATIONAL_V8);
    let first = project.apply(
        project.current().revision(),
        [add(
            "use.kerml",
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
    )?;
    if !first.is_complete_slice() {
        return Err(format!(
            "Accepted-library authored input: {:?}; {:?}",
            first.diagnostics(),
            first.semantic_diagnostics()
        )
        .into());
    }
    let anything = publication.bindings().get(StandardRole::Anything);
    let things = publication.bindings().get(StandardRole::Things);
    assert_eq!(lookup(&project, &["User", "Universe"])?, anything);
    let special = lookup(&project, &["User", "Special"])?;
    let value = lookup(&project, &["User", "value"])?;
    let original = lookup(&project, &["User", "Parent", "original"])?;
    let changed = lookup(&project, &["User", "Changed"])?;
    let renamed = lookup(&project, &["User", "Changed", "renamed"])?;
    let local = lookup(&project, &["Shadow", "Anything"])?;
    let picked = lookup(&project, &["Shadow", "picked"])?;
    let q = first.queries();
    assert!(q.all_specializations(special).value.contains(&anything));
    assert!(q.feature_types(value).value.contains(&anything));
    assert!(q.all_specializations(value).value.contains(&things));
    assert!(q.redefined_features(renamed).value.contains(&original));
    assert!(!q.effective_features(changed).value.contains(&original));
    assert!(
        first
            .references()
            .iter()
            .any(|r| r.specific == picked && r.resolution.value == Resolution::Resolved(local))
    );
    let hidden = q.lookup_path(
        first.root(),
        &QualifiedName {
            absolute: false,
            segments: ["User", "Parent", "hidden"].map(String::from).to_vec(),
        },
    );
    assert_eq!(hidden.completeness, Completeness::Complete);
    assert!(hidden.value.is_empty());
    assert_eq!(q.context().publication_dependency_digest, Some(digest));
    assert_eq!(q.context().pinned_libraries, publication.library_set().pins);
    let stable_ids: Vec<_> = publication.bindings().iter().collect();
    assert_eq!(
        stable_ids,
        second
            .current()
            .queries()
            .context()
            .standard_bindings
            .as_ref()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
    );
    for (_, id) in &stable_ids {
        assert!(std::ptr::eq(
            publication.overlay().model().element(*id).unwrap(),
            first.snapshot().model().element(*id).unwrap()
        ));
    }
    let next = project.apply(
        first.revision(),
        [add(
            "next.kerml",
            "namespace More { private import Base::*; feature another : Anything; }",
        )],
    )?;
    assert!(
        next.is_complete_slice(),
        "{:?}",
        next.semantic_diagnostics()
    );
    assert_eq!(
        next.queries().context().publication_dependency_digest,
        Some(digest)
    );
    assert_eq!(publication.semantic_digest(), digest);
    assert!(second.current().documents().next().is_none());
    let independent = second.apply(second.current().revision(), [add(
        "independent.kerml",
        "namespace Independent { private import Base::*; alias Universe for Anything; feature other : Universe subsets things; }",
    )])?;
    assert!(
        independent.is_complete_slice(),
        "{:?}",
        independent.semantic_diagnostics()
    );
    assert_eq!(lookup(&second, &["Independent", "Universe"])?, anything);
    assert_eq!(
        stable_ids,
        next.queries()
            .context()
            .standard_bindings
            .as_ref()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
    );
    // A dependency cannot be rebound under an earlier interpretation authority.
    assert!(
        SemanticContext::for_snapshot(
            next.snapshot(),
            SemanticOptions {
                baseline_profile: BaselineProfile::OPERATIONAL_V7,
                ..Default::default()
            },
            publication.library_set().pins.clone()
        )
        .is_err()
    );
    // Each reader owns its query caches and shares the same immutable records.
    std::thread::scope(|scope| {
        for revision in [&first, &next, &independent] {
            scope.spawn(move || {
                let q = revision.queries();
                let found = q.lookup_path(
                    revision.root(),
                    &QualifiedName {
                        absolute: false,
                        segments: vec!["Base".into(), "Anything".into()],
                    },
                );
                assert_eq!(found.completeness, Completeness::Complete);
                assert_eq!(found.value.len(), 1);
                assert_eq!(found.value[0].element, anything);
                assert_eq!(q.context().publication_dependency_digest, Some(digest));
            });
        }
    });
    let derived = publication
        .overlay()
        .model()
        .elements()
        .find(|r| matches!(r.origin(), Origin::Derived(_)))
        .expect("publication has derived identities")
        .id();
    for target in [anything, derived] {
        let mut mutation = next.snapshot().change_set();
        mutation.set(
            target,
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String("mutated".into())),
            agq_kernel::provenance::DeclaredOrigin::Authored { source: None },
        );
        assert!(matches!(
            next.snapshot().apply(&mutation),
            Err(agq_kernel::ModelError::ImmutableDependency(_))
        ));
        assert_eq!(
            first.snapshot().model().element(target),
            next.snapshot().model().element(target)
        );
    }
    println!(
        "Authored publication consumption passed: import, alias, specialization, typing, subsetting, redefinition, visibility, shadowing, profile, stable IDs, shared immutable publication"
    );
    Ok(())
}
