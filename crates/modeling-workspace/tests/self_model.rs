//! Accepted self-model dogfooding. Explicitly requested only after foundation readiness.
#[allow(dead_code)]
mod support;
use agq_kerml_text::ProjectChange;
use std::{collections::BTreeSet, path::Path};
use support::*;

#[test]
#[ignore = "requires accepted publication caches; never rebuilds standards"]
fn self_model_revision_edit_preserves_architecture_and_old_queries() {
    let mut workspace = open();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models/agentique");
    let mut documents = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "sysml")
        })
        .collect::<Vec<_>>();
    documents.sort();
    assert_eq!(documents.len(), 6);
    let changes = documents
        .iter()
        .map(|path| ProjectChange::Add {
            path: path.file_name().unwrap().to_str().unwrap().into(),
            language: agq_kerml_text::SourceLanguage::SysMl,
            source: std::fs::read_to_string(path).unwrap(),
        })
        .collect::<Vec<_>>();
    let r1 = workspace
        .apply(workspace.head().revision(), changes)
        .unwrap();
    assert_valid(&r1);
    let before = immutable_signature(&r1);
    let unchanged = [
        ["LanguageArchitecture", "SemanticKernel"],
        ["LanguageArchitecture", "KerMLEngine"],
        ["LanguageArchitecture", "SysMLEngine"],
        ["ExecutionArchitecture", "ExecutionSubsystem"],
    ]
    .map(|path| (path, element(&r1, &path)));
    let original_platform = element(&r1, &["PlatformArchitecture", "ModelingPlatform"]);
    let original_members = {
        let queries = r1.sysml_queries().unwrap();
        let answer = queries.effective_usages(original_platform);
        assert_eq!(
            answer.completeness(),
            agq_kerml_semantics::Completeness::Complete
        );
        answer.value().clone()
    };
    let (path, document) = r1
        .documents()
        .find(|(_, document)| document.source().contains("part def ModelingPlatform"))
        .unwrap();
    let source = document.source();
    let declaration = source.find("part def ModelingPlatform").unwrap();
    let body = declaration + source[declaration..].find('{').unwrap() + 1;
    let r2 = workspace
        .apply(
            r1.revision(),
            [edit(
                &r1,
                path,
                body,
                body,
                "\n        part revisionObserver;\n",
            )],
        )
        .unwrap();
    assert_valid(&r2);
    assert_eq!(immutable_signature(&r1), before);
    assert_ne!(r1.revision(), r2.revision());
    assert_eq!(r2.parent(), Some(r1.revision()));
    assert_eq!(r1.project(), r2.project());
    for (path, identity) in unchanged {
        assert_eq!(element(&r2, &path), identity, "unchanged authored {path:?}");
    }
    for revision in [&r1, &r2] {
        assert_shared(revision);
        let queries = revision.kerml_queries().unwrap();
        for (package, name) in [
            ("LanguageArchitecture", "SemanticKernel"),
            ("LanguageArchitecture", "KerMLEngine"),
            ("LanguageArchitecture", "SysMLEngine"),
            ("PlatformArchitecture", "ModelingPlatform"),
            ("ExecutionArchitecture", "ExecutionSubsystem"),
        ] {
            element(revision, &[package, name]);
        }
        for (member_path, target_path) in [
            (
                ["LanguageArchitecture", "KerMLEngine", "kernelContract"],
                ["LanguageArchitecture", "SemanticKernel"],
            ),
            (
                ["LanguageArchitecture", "SysMLEngine", "kermlContract"],
                ["LanguageArchitecture", "KerMLEngine"],
            ),
            (
                ["PlatformArchitecture", "ModelingPlatform", "workspace"],
                ["PlatformArchitecture", "ProjectWorkspace"],
            ),
            (
                [
                    "PlatformArchitecture",
                    "ProjectWorkspace",
                    "languageQueries",
                ],
                ["LanguageArchitecture", "SysMLEngine"],
            ),
            (
                [
                    "PlatformArchitecture",
                    "ProjectWorkspace",
                    "kermlPublication",
                ],
                ["LanguageArchitecture", "StandardLibraryManager"],
            ),
            (
                [
                    "PlatformArchitecture",
                    "ProjectWorkspace",
                    "systemsPublication",
                ],
                ["LanguageArchitecture", "StandardLibraryManager"],
            ),
            (
                ["PlatformArchitecture", "QueryService", "queryKernel"],
                ["LanguageArchitecture", "SemanticKernel"],
            ),
            (
                [
                    "PlatformArchitecture",
                    "ValidationService",
                    "validationLanguage",
                ],
                ["LanguageArchitecture", "SysMLEngine"],
            ),
            (
                [
                    "ExecutionArchitecture",
                    "ExecutionCompiler",
                    "validationContract",
                ],
                ["PlatformArchitecture", "ValidationService"],
            ),
        ] {
            let member = element(revision, &member_path);
            let target = element(revision, &target_path);
            let answer = queries.direct_feature_types(member);
            assert_eq!(
                answer.completeness,
                agq_kerml_semantics::Completeness::Complete
            );
            assert!(
                answer.value.contains(&target),
                "{member_path:?} lost its dependency on {target_path:?}"
            );
        }
        let sysml = revision.sysml_queries().unwrap();
        for (owner_path, declarations) in [
            (
                ["AgentiqueSystem", "Agentique"],
                &[
                    (
                        "languageSubsystem",
                        ["LanguageArchitecture", "LanguageEngine"],
                    ),
                    ("modeling", ["PlatformArchitecture", "ModelingPlatform"]),
                    ("execution", ["ExecutionArchitecture", "ExecutionSubsystem"]),
                ][..],
            ),
            (
                ["LanguageArchitecture", "LanguageEngine"],
                &[
                    ("semanticKernel", ["LanguageArchitecture", "SemanticKernel"]),
                    ("kermlEngine", ["LanguageArchitecture", "KerMLEngine"]),
                    ("sysmlEngine", ["LanguageArchitecture", "SysMLEngine"]),
                    (
                        "standardLibraries",
                        ["LanguageArchitecture", "StandardLibraryManager"],
                    ),
                ][..],
            ),
            (
                ["ExecutionArchitecture", "ExecutionSubsystem"],
                &[
                    ("compiler", ["ExecutionArchitecture", "ExecutionCompiler"]),
                    ("executionIR", ["ExecutionArchitecture", "ExecutionIR"]),
                    ("simulation", ["ExecutionArchitecture", "SimulationRuntime"]),
                ][..],
            ),
        ] {
            let parts = sysml.owned_usages_of_kind(
                element(revision, &owner_path),
                agq_sysml_semantics::UsageKind::Part,
            );
            assert_eq!(
                parts.completeness(),
                agq_kerml_semantics::Completeness::Complete
            );
            let expected = declarations
                .iter()
                .map(|&(name, _)| element(revision, &[owner_path[0], owner_path[1], name]))
                .collect::<BTreeSet<_>>();
            assert_eq!(
                parts.value().iter().copied().collect::<BTreeSet<_>>(),
                expected,
                "{owner_path:?} composition"
            );
            for &(name, target_path) in declarations {
                let part = element(revision, &[owner_path[0], owner_path[1], name]);
                let definitions = sysml.effective_part_definitions(part);
                assert_eq!(
                    definitions.completeness(),
                    agq_kerml_semantics::Completeness::Complete
                );
                assert!(
                    definitions
                        .value()
                        .contains(&element(revision, &target_path)),
                    "{owner_path:?}::{name}: {definitions:?}"
                );
            }
        }
        for (owner_path, target_paths) in [
            (
                ["PlatformArchitecture", "ProjectWorkspace"],
                &[
                    ["LanguageArchitecture", "StandardLibraryManager"],
                    ["LanguageArchitecture", "SysMLEngine"],
                ][..],
            ),
            (
                ["ExecutionArchitecture", "ExecutionCompiler"],
                &[["PlatformArchitecture", "ValidationService"]][..],
            ),
        ] {
            let parts = sysml.owned_usages_of_kind(
                element(revision, &owner_path),
                agq_sysml_semantics::UsageKind::Part,
            );
            assert_eq!(
                parts.completeness(),
                agq_kerml_semantics::Completeness::Complete
            );
            let mut dependencies = BTreeSet::new();
            for &part in parts.value() {
                if sysml
                    .model()
                    .navigation_slot(part, agq_kerml::properties::FEATURE_IS_COMPOSITE)
                    .is_some_and(|slot| {
                        matches!(
                            slot.value(),
                            agq_kernel::value::SlotValue::Scalar(
                                agq_kernel::value::Value::Boolean(false)
                            )
                        )
                    })
                {
                    let types = queries.direct_feature_types(part);
                    assert_eq!(
                        types.completeness,
                        agq_kerml_semantics::Completeness::Complete
                    );
                    dependencies.extend(types.value);
                }
            }
            let expected = target_paths
                .iter()
                .map(|path| element(revision, path))
                .collect::<BTreeSet<_>>();
            assert_eq!(dependencies, expected, "{owner_path:?} dependencies");
        }
    }
    let added = element(
        &r2,
        &[
            "PlatformArchitecture",
            "ModelingPlatform",
            "revisionObserver",
        ],
    );
    assert!(r1.semantic_model().unwrap().element(added).is_none());
    let old_queries = r1.sysml_queries().unwrap();
    let old_members = old_queries.effective_usages(original_platform);
    assert_eq!(
        old_members.completeness(),
        agq_kerml_semantics::Completeness::Complete
    );
    assert_eq!(old_members.value(), &original_members);
    assert!(!old_members.value().contains(&added));
    let new_queries = r2.sysml_queries().unwrap();
    let new_platform = element(&r2, &["PlatformArchitecture", "ModelingPlatform"]);
    let new_members = new_queries.effective_usages(new_platform);
    assert_eq!(
        new_members.completeness(),
        agq_kerml_semantics::Completeness::Complete
    );
    assert!(new_members.value().contains(&added));
}
