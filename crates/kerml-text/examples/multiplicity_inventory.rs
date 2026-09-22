//! Exact pinned bound population without producer closure or publication claims.
use agq_kerml::BaselineProfile;
use agq_kerml_text::library::lower_declarations_with_profile;
use agq_standard_libraries::VerifiedLibrarySet;
use std::path::Path;
#[path = "support/multiplicity_inventory.rs"]
mod multiplicity_inventory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = std::env::args()
        .find_map(|a| a.strip_prefix("--output=").map(str::to_owned))
        .ok_or("--output is required")?;
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let draft = lower_declarations_with_profile(&sources, BaselineProfile::OPERATIONAL_V9)?;
    let mut report = multiplicity_inventory::collect(
        draft.candidate().model(),
        &draft.queries(&sources)?,
        draft.source_map(),
        &sources,
        None,
        false,
    )?;
    report["historical_population"] =
        multiplicity_inventory::historical_population(&report, &root)?;
    let path = root.join(output);
    std::fs::create_dir_all(path.parent().ok_or("output parent")?)?;
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?,
        &report,
    )?;
    println!(
        "{} bounds: {}; {} retained symbolic references",
        report["multiplicity_ranges"],
        report["counts"],
        report["historical_population"]["previous_count"]
    );
    if report["complete"] != true || report["historical_population"]["complete"] != true {
        return Err("Multiplicity inventory is incomplete; inspect generated findings".into());
    }
    Ok(())
}
