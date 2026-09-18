use agq_metamodel_gen::{baseline, full_audit, pipeline};
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn complete_inventory_reaches_shapes_outside_the_old_slice_without_publishing_them() {
    let bundle = pipeline::generate_profile(&root(), baseline::SYSML).unwrap();
    let audit = full_audit::report(&bundle).unwrap();
    assert_eq!(audit["counts"]["KerML::Class"], 82);
    assert_eq!(audit["counts"]["SysML::Class"], 93);
    assert_eq!(audit["counts"]["KerML::Property"], 313);
    assert_eq!(audit["counts"]["SysML::Property"], 402);
    assert_eq!(audit["association_arities"], serde_json::json!({"2": 319}));
    assert_eq!(audit["result"], "blocked");
    assert_eq!(audit["translation_attempted"], true);
    assert_eq!(audit["registration_attempted"], false);
    assert!(audit["registration_error"].is_null());
    let findings = audit["findings"].as_array().unwrap();
    for (category, count) in [("C", 2), ("D", 1), ("E", 5), ("F", 1)] {
        assert_eq!(
            findings
                .iter()
                .filter(|f| f["category"] == category)
                .count(),
            count
        );
    }
    assert_eq!(
        findings
            .iter()
            .filter(|f| f["category"] == "E" && f["diagnostic"]["disposition"] == "unreviewed")
            .count(),
        0
    );
    let context = findings.iter().find(|f| f["category"] == "F").unwrap();
    assert_eq!(
        context["descriptor_id"],
        "2abb2284-8e25-51ac-b486-1792cc60e1b1"
    );
    assert_eq!(
        context["detail"]["base_descriptor_id"],
        "2e4efe58-2d09-5275-991e-104649d59bf3"
    );
    for (path, bytes) in full_audit::artifacts(&bundle, baseline::SYSML.id).unwrap() {
        assert_eq!(bytes, fs::read(root().join(path)).unwrap());
    }
}

#[test]
fn full_manifest_preserves_all_existing_runtime_identities_and_cross_model_owners() {
    let bundle = pipeline::generate_profile(&root(), baseline::SYSML).unwrap();
    let manifest = full_audit::manifest(&bundle).unwrap();
    let classes = manifest["classifiers"].as_object().unwrap();
    let properties = manifest["properties"].as_object().unwrap();
    let existing = agq_kerml::descriptors();
    for c in existing.classes {
        assert!(
            classes
                .values()
                .any(|v| v["descriptor_id"] == c.id.to_string())
        );
    }
    for p in existing.properties {
        assert!(
            properties
                .values()
                .any(|v| v["descriptor_id"] == p.id.to_string())
        );
    }
    assert_eq!(classes.len(), 501);
    assert_eq!(properties.len(), 715);
    for (id, c) in classes {
        assert!(
            id.starts_with(
                c["entity"]["key"]["source"]["artifact_uri"]
                    .as_str()
                    .unwrap()
            )
        );
    }
    // Full selection also includes disconnected declarations. No fixed seed list
    // or hand-maintained count is allowed to constrain generator selection.
    let mut model = bundle.metamodel.clone();
    let mut extra = model.classifiers.values().next().unwrap().clone();
    extra.entity.key.external_id = "independent-declaration".into();
    model
        .classifiers
        .insert("independent-declaration".into(), extra);
    assert!(
        full_audit::selection(&model)
            .classifiers
            .contains("independent-declaration")
    );
}

#[test]
fn full_audit_mode_is_separate_from_runtime_emission_and_check_is_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let command = || {
        let mut c = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"));
        c.args(["--audit-full", "--descriptor-output"])
            .arg(directory.path());
        c
    };
    let result = command().output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!directory.path().join("crates").exists());
    for baseline in ["kerml-1.0", "sysml-2.0"] {
        assert!(
            !directory
                .path()
                .join(format!("standards/generated/{baseline}/metamodel.json"))
                .exists()
        );
    }
    assert!(command().arg("--check").status().unwrap().success());
    let path = directory
        .path()
        .join("standards/generated/sysml-2.0/full-audit.json");
    fs::write(&path, b"stale full audit").unwrap();
    let result = command().arg("--check").output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("generated output is stale"));
    assert_eq!(fs::read(path).unwrap(), b"stale full audit");
}

#[test]
fn readiness_cannot_certify_the_bounded_kerml_registry_as_complete() {
    let directory = tempfile::tempdir().unwrap();
    for baseline in ["kerml-1.0", "sysml-2.0"] {
        let result = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"))
            .args([
                "--baseline",
                baseline,
                "--require-runtime",
                "--check",
                "--descriptor-output",
            ])
            .arg(directory.path())
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("complete runtime blocked"));
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }
}
