//! Both explicit grammar profiles over original verified Systems KPAR bytes.
use agq_kerml_syntax::production::{self, Limits, SysmlSyntaxProfile};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::json;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let libraries = VerifiedLibrarySet::load_from_directory(&root)?;
    let mut profiles = Vec::new();
    for profile in [
        SysmlSyntaxProfile::Published,
        SysmlSyntaxProfile::OperationalV1,
    ] {
        let mut reports = Vec::new();
        let mut parsed = 0;
        for source in libraries
            .documents()
            .filter(|d| d.language() == LibraryLanguage::SysMl)
        {
            let doc = production::parse_sysml_with_profile(
                profile,
                source.document(),
                source.revision(),
                source.source(),
                Limits::default(),
            )?;
            let exact_bytes = doc
                .tokens()
                .iter()
                .map(|t| doc.token_text(t))
                .collect::<String>()
                == source.source();
            assert!(exact_bytes);
            parsed += usize::from(doc.is_complete());
            let diagnostics: Vec<_> = doc
                .diagnostics()
                .iter()
                .map(|d| {
                    json!({
                        "code": d.code, "byte_range": [d.range.start(), d.range.end()],
                        "token": doc.text(d.range), "message": d.message,
                    })
                })
                .collect();
            reports.push(json!({
            "path": source.path(), "profile": profile.id(), "sha256": source.sha256(), "document": source.document().to_string(),
            "revision": source.revision().to_string(), "bytes": source.source().len(),
            "exact_byte_preservation": exact_bytes, "complete": doc.is_complete(),
            "production_nodes": doc.nodes().count(), "recovery_regions": doc.recovery().len(),
            "root_productions": doc.roots().map(|node| node.kind().name()).collect::<Vec<_>>(),
            "diagnostics": diagnostics,
        }));
        }
        reports.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        profiles.push(json!({
        "profile": profile.id(),
        "grammar_compatibility_manifest_sha256": profile.grammar_compatibility_manifest_sha256(),
        "documents_parsed": parsed, "documents_total": reports.len(), "documents": reports,
    }));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "format": "agentique-sysml-frontend-profiles/1", "grammar": "SysML 2.0 final",
            "profiles": profiles,
            "semantic_acceptance": false,
        }))?
    );
    Ok(())
}
