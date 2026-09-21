//! Complete local owned-cross-feature audit; unresolved references remain explicit.
use agq_kerml::{BaselineProfile, classes as c};
use agq_kerml_semantics::Completeness;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{io::Write, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let profile = if std::env::args().any(|a| a == "--v6") {
        BaselineProfile::OPERATIONAL_V6
    } else {
        BaselineProfile::OPERATIONAL_V7
    };
    let draft = agq_kerml_text::library::lower_declarations_with_profile(&sources, profile)?;
    let model = draft.candidate().model();
    let queries = draft.queries(&sources)?;
    let mut rows = vec![];
    for record in model.elements() {
        if !model
            .registry()
            .is_subtype(record.metaclass(), c::FEATURE)?
        {
            continue;
        }
        let result = queries.owned_cross_feature(record.id());
        let owned = queries.memberships(record.id());
        rows.push(json!({
            "feature":record.id().to_string(),
            "owned_memberships":owned.value.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "selected":result.value.map(|id|id.to_string()),
            "completeness":format!("{:?}",result.completeness),
            "dependency_count":result.positive_dependencies.len(),
            "search_dependency_count":result.search_dependencies.len(),
        }));
        if result.completeness != Completeness::Complete {
            return Err(format!("Incomplete local ownership: {:?}", result.diagnostics).into());
        }
    }
    let report = json!({
        "format":"agentique-owned-cross-feature-corpus/1", "profile":profile.id(),
        "library_set":sources.content_set_id(),"context":format!("{:?}",queries.context()),
        "canonical_record_count":model.elements().count(),
        "required_reference_count":draft.references().len(),
        "unresolved_storage_obligations":draft.candidate().obligations().len(),
        "scope":"All Features in all 36 pinned documents. Local selector only; no claim of full structural expansion or publication.",
        "rows":rows,
    });
    let path = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?
        .write_all(format!("{}\n", serde_json::to_string_pretty(&report)?).as_bytes())?;
    println!(
        "{}: {} Features checked in 36 documents",
        profile.id(),
        rows.len()
    );
    Ok(())
}
