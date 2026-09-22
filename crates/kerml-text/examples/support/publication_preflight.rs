//! A whole-corpus attempt requires five successful preflights over this exact input.
use agq_kerml_semantics::SemanticContextId;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::Path};

pub fn identity(context: &SemanticContextId, source_content_set: &str) -> Value {
    json!({"profile":context.baseline_profile_id, "rule_set":context.rule_set_version,
        "source_content_set":source_content_set, "descriptor_digest":context.descriptor_digest,
        "declared_graph_digest":context.model_digest})
}

pub fn check(
    directory: &Path,
    expected: &Value,
) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut references = BTreeSet::new();
    for label in ["A", "B", "C", "D", "E"] {
        let report: Value = serde_json::from_slice(&std::fs::read(
            directory.join(format!("slice-{label}.json")),
        )?)?;
        if !accepted(&report, label, expected) {
            return Err(format!(
                "Slice {label} has not passed the complete preflight for this publication input"
            )
            .into());
        }
        references.extend(complete_references(&report)?);
    }
    Ok(references)
}

fn complete_references(report: &Value) -> Result<BTreeSet<String>, Box<dyn std::error::Error>> {
    let mut references = BTreeSet::new();
    for bound in report["multiplicity_bounds"]["bounds"]
        .as_array()
        .ok_or("Slice bound inventory is missing")?
    {
        for reference in bound["references"]
            .as_array()
            .ok_or("Slice bound reference inventory is missing")?
        {
            if reference["complete"] != true {
                return Err("Slice symbolic reference is not Complete".into());
            }
            references.insert(
                reference["expression"]
                    .as_str()
                    .ok_or("Slice symbolic reference identity is missing")?
                    .to_owned(),
            );
        }
    }
    let declared: BTreeSet<String> = report["multiplicity_bounds"]["reference_expression_ids"]
        .as_array()
        .ok_or("Slice reference population is missing")?
        .iter()
        .map(|id| {
            id.as_str()
                .map(str::to_owned)
                .ok_or("Invalid slice reference identity")
        })
        .collect::<Result<_, _>>()?;
    if references != declared {
        return Err("Slice symbolic reference population disagrees with its audited bounds".into());
    }
    Ok(references)
}

fn accepted(report: &Value, label: &str, expected: &Value) -> bool {
    report["format"] == "agentique-publication-slice/2"
        && report["slice"] == label
        && report["input_identity"] == *expected
        && report["passed"] == true
        && report["converged"] == true
        && report["producer_completeness"] == "Complete"
        && report["scope_boundary"]["complete"] == true
        && report["resource_stop"] == false
        && report["capability_failures"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && report["reference_failures"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && report["mandatory_references_checked"]
            .as_u64()
            .is_some_and(|n| n > 0)
        && report["multiplicity_bounds"]["semantic_audit_executed"] == true
        && report["multiplicity_bounds"]["complete"] == true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_rejects_incomplete_or_stale_evidence() {
        let identity =
            json!({"profile":"v9", "rule_set":"query/current", "declared_graph_digest":"exact"});
        let passed = json!({"format":"agentique-publication-slice/2", "slice":"A", "input_identity":identity,
            "passed":true, "converged":true, "producer_completeness":"Complete", "scope_boundary":{"complete":true},
            "resource_stop":false, "capability_failures":[], "reference_failures":[], "mandatory_references_checked":12,
            "multiplicity_bounds":{"semantic_audit_executed":true,"complete":true}});
        assert!(accepted(&passed, "A", &identity));
        assert!(!accepted(&passed, "B", &identity));
        for (field, value) in [
            ("input_identity", json!({"profile":"v8"})),
            ("passed", json!(false)),
            ("converged", json!(false)),
            ("producer_completeness", json!("Incomplete")),
            ("scope_boundary", json!({"complete":false})),
            ("resource_stop", json!(true)),
            ("capability_failures", json!(["finding"])),
            ("reference_failures", json!(["incomplete"])),
            ("mandatory_references_checked", json!(0)),
            ("multiplicity_bounds", json!({"complete":true})),
        ] {
            let mut failed = passed.clone();
            failed[field] = value;
            assert!(
                !accepted(&failed, "A", &identity),
                "incorrectly accepted {field}"
            );
        }
    }

    #[test]
    fn symbolic_preflight_uses_complete_audited_ids_instead_of_summary_flags() {
        let report = json!({"multiplicity_bounds":{"complete":true,"reference_expression_ids":["a","b"],
            "bounds":[{"references":[{"expression":"a","complete":true}]},{"references":[{"expression":"b","complete":true}]}]}});
        assert_eq!(
            complete_references(&report).unwrap(),
            BTreeSet::from(["a".into(), "b".into()])
        );
        let mut incomplete = report.clone();
        incomplete["multiplicity_bounds"]["bounds"][1]["references"][0]["complete"] = json!(false);
        assert!(complete_references(&incomplete).is_err());
        let mut missing = report.clone();
        missing["multiplicity_bounds"]["bounds"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(complete_references(&missing).is_err());
    }
}
