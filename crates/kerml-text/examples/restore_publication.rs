//! Receipt-bound restoration and authored consumption, without corpus closure.
use agq_kerml_text::library::CanonicalKermlStandardLibraries;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::json;
use std::{path::Path, sync::Arc};

#[path = "support/authored_publication.rs"]
mod authored_publication;
#[path = "support/multiplicity_inventory.rs"]
mod multiplicity_inventory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cache = std::env::args()
        .find_map(|arg| arg.strip_prefix("--cache=").map(|path| root.join(path)))
        .ok_or("--cache=<accepted publication ZIP> is required")?;
    let output = std::env::args()
        .find_map(|arg| arg.strip_prefix("--output=").map(|path| root.join(path)))
        .ok_or("--output=<fresh verification JSON> is required")?;
    if output.exists() {
        return Err("Output exists; use a fresh verification path".into());
    }
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    println!(
        "restoring checked-in accepted publication: {}",
        cache.display()
    );
    let publication = Arc::new(CanonicalKermlStandardLibraries::restore_cache(
        std::fs::File::open(&cache)?,
        &sources,
    )?);
    let authored = authored_publication::verify(publication.clone());
    let bounds = if std::env::args().any(|arg| arg == "--audit-bounds") {
        println!("Multiplicity inventory: auditing restored accepted publication");
        let mut report = multiplicity_inventory::collect(
            publication.overlay().model(),
            &publication.queries(),
            publication.source_map(),
            &sources,
            None,
            true,
        )?;
        report["historical_population"] =
            multiplicity_inventory::historical_population(&report, &root)?;
        Some(report)
    } else {
        None
    };
    let bounds_complete = bounds.as_ref().is_none_or(|report| {
        report["complete"] == true && report["historical_population"]["complete"] == true
    });
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&json!({
            "format":"agq-kerml-restored-publication-check/1",
            "cache":cache,
            "restored_from_trusted_receipt":publication.complete_overlay().restored_from_receipt(),
            "operational_profile":publication.profile().id(),
            "rule_set":publication.context().rule_set_version,
            "semantic_digest":publication.semantic_digest(),
            "source_content_set":publication.source_content_set(),
            "roots":publication.roots().len(),
            "source_map_entries":publication.source_map().len(),
            "mandatory_references":publication.mandatory_reference_count(),
            "standard_roles":publication.bindings().iter().count(),
            "producer_closure_rerun":false,
            "authored_consumption_verified":authored.is_ok(),
            "authored_failure":authored.as_ref().err().map(ToString::to_string),
            "multiplicity_bounds":bounds,
        }))?,
    )?;
    authored?;
    if !bounds_complete {
        return Err("Accepted publication multiplicity audit is incomplete".into());
    }
    println!("Accepted publication restored; authored consumption verified");
    Ok(())
}
