use agq_workspace::Revision;
use serde_json::Value;
use std::collections::BTreeMap;
#[test]
fn at_self01_trace01_registered_declarations() {
    let r = Revision::import(
        BTreeMap::from([
            (
                "models/AgentiqueArchitecture.sysml".into(),
                include_str!("../../../models/AgentiqueArchitecture.sysml").into(),
            ),
            (
                "models/AgentiqueBehaviour.sysml".into(),
                include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
            ),
            (
                "models/AgentiqueRequirements.sysml".into(),
                include_str!("../../../models/AgentiqueRequirements.sysml").into(),
            ),
            (
                "models/AgentiqueVerification.sysml".into(),
                include_str!("../../../models/AgentiqueVerification.sysml").into(),
            ),
        ]),
        &BTreeMap::new(),
    )
    .unwrap();
    let requirements: Vec<Value> =
        serde_json::from_str(include_str!("../../../requirements.json")).unwrap();
    assert_eq!(requirements.len(), 20);
    for requirement in requirements {
        let element = r
            .model
            .by_path(requirement["model_ref"].as_str().unwrap())
            .unwrap();
        assert_eq!(element.kind, "RequirementUsage");
        assert_eq!(element.short_name.as_deref(), requirement["id"].as_str());
        assert!(
            element
                .documentation
                .contains(requirement["requirement"].as_str().unwrap())
        );
        let verification = r
            .model
            .by_path(&format!(
                "AgentiqueVerification::verify_{}",
                requirement["name"].as_str().unwrap()
            ))
            .unwrap();
        assert_eq!(verification.kind, "VerificationCaseDefinition");
        assert!(r.model.elements.values().any(|e| {
            e.qualified_name.starts_with(&verification.qualified_name)
                && e.target("requirement") == Some(&element.id)
        }));
    }
}
