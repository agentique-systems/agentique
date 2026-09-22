//! A whole-corpus attempt requires five successful preflights over this exact input.
use agq_kerml_semantics::SemanticContextId;
use serde_json::{Value, json};
use std::path::Path;

pub fn identity(context: &SemanticContextId, source_content_set: &str) -> Value {
    json!({"profile":context.baseline_profile_id, "rule_set":context.rule_set_version,
        "source_content_set":source_content_set, "descriptor_digest":context.descriptor_digest,
        "declared_graph_digest":context.model_digest})
}

pub fn check(directory: &Path, expected: &Value) -> Result<(), Box<dyn std::error::Error>> {
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
    }
    Ok(())
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
}
