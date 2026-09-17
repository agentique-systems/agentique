use agq_metamodel_gen::{canonical_json, ir::*, pipeline};
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn pinned_normative_artifacts_import_cross_check_and_match_committed_ir() {
    let bundle = pipeline::generate(&root()).unwrap();
    assert_eq!(bundle.cross_check.classes, 82);
    assert_eq!(bundle.cross_check.enumerations, 2);
    assert_eq!(bundle.cross_check.owned_properties, 210);
    assert_eq!(bundle.metamodel.packages.len(), 24);
    assert_eq!(bundle.metamodel.properties.len(), 313);
    assert_eq!(bundle.primitive_types.classifiers.len(), 5);
    assert_eq!(bundle.cross_check.direct_generalizations, 88);
    assert_eq!(
        bundle
            .cross_check
            .nullable_data_scalars_with_positive_xmi_lower
            .len(),
        36
    );
    assert_eq!(
        bundle
            .metamodel
            .properties
            .values()
            .map(|p| p.redefines.len())
            .sum::<usize>(),
        79
    );
    assert_eq!(
        bundle
            .metamodel
            .properties
            .values()
            .map(|p| p.subsets.len())
            .sum::<usize>(),
        207
    );
    assert_eq!(
        bundle
            .metamodel
            .properties
            .values()
            .filter(|p| p.is_derived_union)
            .count(),
        3
    );
    assert_eq!(
        bundle
            .metamodel
            .properties
            .values()
            .filter(|p| p.entity.name.is_empty())
            .count(),
        3
    );
    let default = bundle.metamodel.properties["Core-Features-Feature-isUnique"]
        .default_value
        .as_ref()
        .unwrap();
    assert_eq!(default.attributes["value"], "true");
    assert!(bundle.metamodel.properties["Root-Elements-Element-elementId"].is_id);
    let association = &bundle.metamodel.classifiers["Kernel-Associations-Association"];
    assert_eq!(association.generalizations.len(), 2);
    assert!(
        bundle
            .metamodel
            .properties
            .values()
            .all(|p| p.aggregation == Aggregation::None)
    );
    let first = canonical_json(&bundle).unwrap();
    assert_eq!(first, pipeline::bytes(&root()).unwrap());
    assert_eq!(first, fs::read(root().join(pipeline::OUTPUT_PATH)).unwrap());
}

#[test]
fn cli_check_is_read_only_and_detects_stale_output_and_tampered_inputs() {
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("model.json");
    let binary = env!("CARGO_BIN_EXE_metamodel-gen");
    let run = |extra: &[&str]| {
        Command::new(binary)
            .arg("--output")
            .arg(&output)
            .args(extra)
            .output()
            .unwrap()
    };
    assert!(run(&[]).status.success());
    assert!(run(&["--check"]).status.success());
    fs::write(&output, b"stale\n").unwrap();
    let result = run(&["--check"]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("stale"));
    assert_eq!(fs::read(&output).unwrap(), b"stale\n");
    let inputs = temporary.path().join("standards/normative/kerml-1.0");
    fs::create_dir_all(&inputs).unwrap();
    for name in ["lock.json", "KerML.xmi", "KerML.json", "PrimitiveTypes.xmi"] {
        fs::copy(
            root().join("standards/normative/kerml-1.0").join(name),
            inputs.join(name),
        )
        .unwrap();
    }
    let original = fs::read(inputs.join("KerML.json")).unwrap();
    let refusal = Command::new(binary)
        .arg("--root")
        .arg(temporary.path())
        .arg("--output")
        .arg(inputs.join("KerML.json"))
        .output()
        .unwrap();
    assert!(!refusal.status.success());
    assert!(String::from_utf8_lossy(&refusal.stderr).contains("must not overwrite normative"));
    assert_eq!(fs::read(inputs.join("KerML.json")).unwrap(), original);
    fs::write(inputs.join("KerML.xmi"), b"tampered").unwrap();
    let result = Command::new(binary)
        .arg("--root")
        .arg(temporary.path())
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("integrity mismatch"));
    assert_eq!(fs::read(&output).unwrap(), b"stale\n");
}
