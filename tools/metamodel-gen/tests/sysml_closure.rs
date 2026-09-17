use agq_metamodel_gen::{baseline, closure_audit, descriptors, graph, pipeline};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn minimum_sysml_closure_retains_and_diagnoses_published_self_subsets() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bundle = pipeline::generate_profile(&root, baseline::SYSML).unwrap();
    let audit = closure_audit::report(&bundle).unwrap();
    assert_eq!(audit["result"], "blocked");
    assert_eq!(audit["classifiers"]["SysML::Class"], 54);
    assert_eq!(audit["classifiers"]["KerML::Class"], 50);
    assert_eq!(audit["properties"], 562);
    assert_eq!(
        audit["same_closure_with_attributes_ports_connections"],
        true
    );
    let issues = audit["self_subsetting_properties"].as_array().unwrap();
    assert_eq!(issues.len(), 2);
    for (issue, expected) in issues.iter().zip([
        "https://www.omg.org/spec/KerML/20250201/KerML.xmi#Kernel-Associations-A_targetType_targetAssociation-targetAssociation",
        "https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-DefinitionAndUsage-A_analysisCaseOwningUsage_nestedAnalysisCase-analysisCaseOwningUsage",
    ]) {
        assert_eq!(issue["source_qualified_id"], expected);
        assert!(issue["subsets"].as_array().unwrap().iter().any(|v| v == expected));
        assert_eq!(issue["required_by"].as_array().unwrap().last().unwrap(), expected);
    }
    let bytes = closure_audit::bytes(&bundle).unwrap();
    assert_eq!(bytes, closure_audit::bytes(&bundle).unwrap());
    assert_eq!(bytes, fs::read(root.join(closure_audit::PATH)).unwrap());
    let g = graph::combine(&bundle.metamodel, &bundle.dependencies).unwrap();
    let selected = descriptors::closure(
        &g,
        &["https://www.omg.org/spec/SysML/20250201/SysML.xmi#Systems-Parts-PartDefinition"],
    )
    .unwrap();
    // The source-qualified graph keeps the exact dependency metamodel identity.
    for id in selected.classifiers {
        let key = &g.classifiers[&id].entity.key;
        assert!(id.starts_with(&format!("{}#", key.source.artifact_uri)));
    }
}

#[test]
fn runtime_readiness_gate_fails_without_changing_outputs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = root.join(baseline::SYSML.output.unwrap());
    let before = fs::read(&output).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_metamodel-gen"))
        .args(["--baseline", "sysml-2.0", "--require-runtime", "--check"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("SysML runtime closure blocked"));
    assert_eq!(before, fs::read(output).unwrap());
}
