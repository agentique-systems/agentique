//! Recovery wrapper for the frozen a1a7b847 producer, not a new publication authority.
//! Copy into that checkout's crates/kerml-text/examples before building.
use agq_kerml::BaselineProfile;
use agq_kerml_semantics::AuthorityImpact;
use agq_kerml_text::library::CanonicalKermlStandardLibraries;
use agq_standard_libraries::VerifiedLibrarySet;
use serde_json::{Value, json};
use std::{path::Path, time::Instant};

#[path = "support/publication_authority.rs"]
mod publication_authority;
#[path = "support/publication_output.rs"]
mod publication_output;
#[path = "support/publication_refinement.rs"]
mod publication_refinement;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        return Err("Recovery requires the release producer".into());
    }
    let argument =
        |prefix: &str| std::env::args().find_map(|arg| arg.strip_prefix(prefix).map(str::to_owned));
    let authority = argument("--authority-root=").ok_or("--authority-root is required")?;
    let authority = Path::new(&authority);
    let receipt: Value = serde_json::from_slice(&std::fs::read(
        authority.join("standards/kerml-accepted-publication.json"),
    )?)?;
    let bindings: Value = serde_json::from_slice(&std::fs::read(
        authority.join("standards/kerml-standard-bindings.json"),
    )?)?;
    if receipt["status"] != "accepted"
        || receipt["complete_overlay"]["identity"]["operational_profile"]
            != BaselineProfile::OPERATIONAL_V9.id()
    {
        return Err("Existing accepted v9 authority is required".into());
    }
    let output = argument("--output=").ok_or("--output is required")?;
    let output = Path::new(&output);
    std::fs::create_dir_all(output.parent().ok_or("output parent")?)?;
    let report = publication_output::ReservedFile::new(output)?;
    let cache = publication_output::ReservedFile::new(output.with_extension("publication.zip"))?;
    let generated_receipt =
        publication_output::ReservedFile::new(output.with_extension("publication.receipt.json"))?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root)?;
    if receipt["source_content_set"] != sources.content_set_id() {
        return Err("Pinned source identity differs from existing acceptance".into());
    }
    if publication_authority::conflicts(&root)?
        .values()
        .any(|impact| *impact == AuthorityImpact::PublicationBlockingAuthorityConflict)
    {
        return Err("Original publication authority contains a blocking conflict".into());
    }
    let started = Instant::now();
    let (draft, refinement) = publication_refinement::prepare(&sources)?;
    if json!(draft.queries(&sources)?.context().rule_set_version)
        != receipt["complete_overlay"]["identity"]["rule_set"]
    {
        return Err("Frozen producer rule identity differs from acceptance".into());
    }
    let publication = CanonicalKermlStandardLibraries::publish(
        draft,
        &sources,
        16,
        |stage, completed, total, records| {
            if completed % 512 == 0 || completed == total {
                println!("producer stage {stage}: {completed}/{total}, {records} proposed records");
            }
        },
        |stage| {
            println!(
                "stage {} {:?}: {} new elements; {:?}",
                stage.stage, stage.stratum, stage.added_elements, stage.completeness
            )
        },
        |done, failures| {
            if done % 512 == 0 {
                println!("references: {done}, failures: {failures}");
            }
        },
        |done, total, failures| {
            if done % 512 == 0 || done == total {
                println!("capabilities: {done}/{total}, findings: {failures}");
            }
        },
    )?;
    if json!(publication.semantic_digest())
        != receipt["complete_overlay"]["identity"]["semantic_digest"]
    {
        return Err(
            "Real semantic mismatch with the accepted publication; stop runtime acceptance".into(),
        );
    }
    publication.check_binding_manifest(&sources, &bindings)?;
    if publication.binding_manifest(&sources)? != bindings {
        return Err("Accepted bindings differ; stop runtime acceptance".into());
    }
    println!(
        "Existing accepted semantic identity and all 31 bindings match; writing candidate cache"
    );
    let candidate = publication.write_cache(cache.file(), &sources)?;
    cache.file().sync_all()?;
    generated_receipt.write_json(&candidate)?;
    report.write_json(&json!({
        "format":"agq-accepted-cache-rematerialization/1",
        "authority_status":"candidate-pending-original-receipt-restoration",
        "historical_producer_commit":"a1a7b847ef83f8e4c54bea9242cac94a9ce665fe",
        "semantic_identity_equal":true,"binding_manifest_equal":true,
        "semantic_digest":publication.semantic_digest(),
        "references":publication.mandatory_reference_count(),
        "source_content_set":sources.content_set_id(),
        "elapsed_seconds":started.elapsed().as_secs_f64(),
        "reference_refinement":refinement,
        "receipt_changed":false,
        "full_closure_and_capability_audits_executed":true,
        "development_slice_preflights_repeated":false
    }))?;
    Ok(())
}
