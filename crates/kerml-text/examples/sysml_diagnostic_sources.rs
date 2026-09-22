//! Map exact standard-library canonical syntax IDs without replaying producers.
use agq_kerml_syntax::production::{self, SysmlSyntaxProfile};
use agq_standard_libraries::{LibraryElementRole, LibraryLanguage, VerifiedLibrarySet};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report: serde_json::Value = serde_json::from_slice(&std::fs::read(
        std::env::args().nth(1).ok_or("report path is required")?,
    )?)?;
    let wanted: BTreeSet<_> = report["last_producer_stage"]["diagnostics"]
        .as_array()
        .ok_or("last producer diagnostics missing")?
        .iter()
        .filter_map(|diagnostic| diagnostic["subject"].as_str())
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    let mut found = BTreeSet::new();
    for source in sources
        .documents()
        .filter(|s| s.language() == LibraryLanguage::SysMl)
    {
        let syntax = production::parse_sysml_with_profile(
            SysmlSyntaxProfile::OperationalV2,
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        )?;
        let mut locators = BTreeMap::new();
        for node in syntax.nodes() {
            *locators
                .entry((node.range().start(), node.range().end(), node.kind().name()))
                .or_insert(0u32) += 1;
        }
        for node in syntax.nodes() {
            // Canonical lowering assigns ordinals only within identical byte
            // locators and production roles. Counting all syntax candidates is
            // a safe upper bound; matching an ID never asserts it was lowered.
            for ordinal in
                0..locators[&(node.range().start(), node.range().end(), node.kind().name())]
            {
                let id = source.element_id(
                    node.range(),
                    LibraryElementRole::Canonical {
                        role: node.kind().name(),
                        ordinal,
                    },
                )?;
                if wanted.contains(id.to_string().as_str()) && found.insert(id.to_string()) {
                    println!(
                        "{}",
                        serde_json::json!({
                            "subject": id,
                            "document": source.path(),
                            "production": node.kind().name(),
                            "start": node.range().start(),
                            "end": node.range().end(),
                            "source": node.text(),
                        })
                    );
                }
            }
        }
    }
    for subject in wanted.iter().filter(|subject| !found.contains(**subject)) {
        eprintln!(
            "No declared Systems syntax locator for {subject}; derived records require graph provenance"
        );
    }
    Ok(())
}
