//! Full production frontend over the exact offline three-library KerML corpus.
use agq_kerml_syntax::production::{self, Limits};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::json;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let set = VerifiedLibrarySet::load_from_directory(&root)?;
    let mut documents = vec![];
    let mut passed = true;
    for source in set
        .documents()
        .filter(|d| d.language() == LibraryLanguage::KerMl)
    {
        let parsed = production::parse(
            source.document(),
            source.revision(),
            source.source(),
            Limits::default(),
        )?;
        let round_trip: String = parsed
            .tokens()
            .iter()
            .map(|t| parsed.token_text(t))
            .collect();
        let mut end = 0;
        for token in parsed.tokens() {
            assert_eq!(token.range.start(), end);
            end = token.range.end();
        }
        assert_eq!(end as usize, source.source().len());
        assert_eq!(round_trip, source.source());
        passed &= parsed.is_complete();
        let syntax: Vec<_> = parsed.diagnostics().iter().map(|d| json!({"code":d.code,"range":[d.range.start(),d.range.end()],"message":d.message})).collect();
        let anomalies: Vec<_> = parsed.discrepancies().iter().map(|d| json!({"code":d.code,"range":[d.range.start(),d.range.end()],"production":d.production.name(),"authority":d.authority})).collect();
        println!(
            "{}: {} tokens, {} syntax nodes, {} recovery ranges, {} grammar discrepancies",
            source.path(),
            parsed.tokens().len(),
            parsed.nodes().count(),
            parsed.recovery().len(),
            anomalies.len()
        );
        documents.push(json!({
            "file":source.path(), "sha256":source.sha256(),
            "document_id":source.document().to_string(), "source_revision_id":source.revision().to_string(),
            "syntax_status":if parsed.is_complete() {"parsed"} else {"recovered"},
            "tokens":parsed.tokens().len(), "syntax_nodes":parsed.nodes().count(),
            "parser_diagnostics":syntax.len(), "recovery_count":parsed.recovery().len(),
            "unsupported_syntax":syntax.len(), "source_coverage_percent":100,
            "exact_source_preservation":true, "canonical_element_count":null,
            "canonical_relationship_count":null, "resolution_status":null,
            "unresolved_count":null, "ambiguous_count":null, "KerML_semantic_status":null,
            "unevaluated_expression_count":null,
            "diagnostics_by_category":{"syntax":syntax,"published_source_anomaly":anomalies,"resolution":null,"KerML_semantic":null,"unevaluated_expression_function_semantics":null}
        }));
    }
    documents.sort_by_key(|d| d["file"].as_str().unwrap().to_owned());
    assert_eq!(documents.len(), 36);
    let report = json!({"format":"agentique-kerml-library-syntax-quality/1", "library_set":set.content_set_id(),
        "scope":"36 pinned KerML documents; syntax only. Null means not evaluated.",
        "syntax_quality_gate_passed":passed,"semantic_quality_gate_passed":false,"documents":documents});
    let args: Vec<_> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--output") {
        let path = args.get(i + 1).ok_or("missing output path")?;
        let bytes = format!("{}\n", serde_json::to_string_pretty(&report)?);
        if args.iter().any(|a| a == "--check") {
            if std::fs::read(path)? != bytes.as_bytes() {
                return Err("stale syntax report".into());
            }
        } else {
            use std::io::Write;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?
                .write_all(bytes.as_bytes())?;
        }
    }
    if !passed || args.iter().any(|a| a == "--require-semantic") {
        std::process::exit(1);
    }
    Ok(())
}
