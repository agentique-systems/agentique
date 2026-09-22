//! Current publication impact, separate from historical issue classifications.
use agq_kerml::BaselineProfile;
use agq_kerml_semantics::AuthorityImpact;
use std::{collections::BTreeMap, path::Path};

pub fn conflicts(
    root: &Path,
) -> Result<BTreeMap<String, AuthorityImpact>, Box<dyn std::error::Error>> {
    let record: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("verification/summaries/kerml-v9-publication/authority-decision.json"),
    )?)?;
    if record["profile"] != BaselineProfile::OPERATIONAL_V9.id() {
        return Err("Publication authority profile mismatch".into());
    }
    let mut conflicts = BTreeMap::new();
    for decision in record["decisions"]
        .as_array()
        .ok_or("authority decisions")?
    {
        let impact = match decision["impact"].as_str() {
            Some("PublicationBlockingAuthorityConflict") => {
                AuthorityImpact::PublicationBlockingAuthorityConflict
            }
            Some("ValidationOnlyAuthorityConflict") => {
                AuthorityImpact::ValidationOnlyAuthorityConflict
            }
            Some("CoveredByOperationalV8" | "CoveredByOperationalV9") => continue,
            _ => return Err("Unknown publication authority impact".into()),
        };
        let issue = decision["issue"].as_str().ok_or("authority issue")?;
        if conflicts.insert(issue.to_owned(), impact).is_some() {
            return Err(format!("Duplicate publication authority issue {issue}").into());
        }
    }
    let blockers = conflicts
        .values()
        .filter(|&&impact| impact == AuthorityImpact::PublicationBlockingAuthorityConflict)
        .count();
    if record["publication_blocking_authority_conflicts"].as_u64() != Some(blockers as u64) {
        return Err("Publication authority blocker count mismatch".into());
    }
    Ok(conflicts)
}
