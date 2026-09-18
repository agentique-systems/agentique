//! Reproducible corpus scan. Lexical candidates are not claimed as grammar productions.
use agq_kerml_syntax::{ParseLimits, TokenKind};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let libraries = VerifiedLibrarySet::load_from_directory(&root)?;
    let mut constructs: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut documents = vec![];
    let mut quality = vec![];
    for doc in libraries.documents() {
        // KerML probing exposes the current frontend's exact gaps. For SysML,
        // report frontend-unavailable; never label this a SysML parser result.
        let syntax = agq_kerml_syntax::parse(doc.document(), doc.source(), ParseLimits::default())?;
        let reconstructed: String = syntax
            .tokens()
            .iter()
            .map(|t| syntax.token_text(t))
            .collect();
        assert_eq!(reconstructed, doc.source());
        let mut end = 0;
        for token in syntax.tokens() {
            assert_eq!(token.range.start(), end);
            end = token.range.end();
            assert!(doc.source().is_char_boundary(end as usize));
            let text = syntax.token_text(token);
            let category = match token.kind {
                TokenKind::Whitespace => "trivia/whitespace".to_string(),
                TokenKind::Note => "trivia/note".to_string(),
                TokenKind::Comment => "annotation/comment-body".to_string(),
                TokenKind::QuotedName => "name/unrestricted".to_string(),
                TokenKind::StringValue => "expression/string-literal".to_string(),
                TokenKind::DecimalValue => "number/decimal-value".to_string(),
                TokenKind::ExponentialValue => "number/exponential-value".to_string(),
                TokenKind::Invalid => format!("unsupported-lexical/{text}"),
                TokenKind::Symbol => format!("symbol/{text}"),
                TokenKind::Word => {
                    if KEYWORDS.split_whitespace().any(|k| k == text) {
                        format!("keyword/{text}")
                    } else {
                        "name/basic".to_string()
                    }
                }
            };
            constructs.entry(category).or_default().push(
                json!({"file":doc.path(),"byte_range":[token.range.start(),token.range.end()]}),
            );
        }
        assert_eq!(end, doc.source().len() as u64);
        let diagnostics: Vec<_> = syntax.diagnostics().iter().map(|d|json!({"code":d.code,"byte_range":[d.range.start(),d.range.end()],"message":d.message})).collect();
        let lexical = syntax
            .diagnostics()
            .iter()
            .filter(|d| d.code == "KS_LEXICAL")
            .count();
        let sysml = doc.language() == LibraryLanguage::SysMl;
        documents.push(json!({
            "file":doc.path(),"sha256":doc.sha256(),"library_id":doc.library().to_string(),
            "document_id":doc.document().to_string(),"source_revision_id":doc.revision().to_string(),
            "bytes":doc.source().len(),"token_count":syntax.tokens().len(),"lossless_token_partition":true,
            "parser":if sysml {"SysML frontend unavailable; KerML probe only"} else {"agq-kerml-syntax"},
            "kerML_probe_diagnostics":diagnostics,
        }));
        quality.push(json!({
            "file":doc.path(),"sha256":doc.sha256(),"lexical_probe_errors":lexical,
            "parse_status":if sysml {"frontend-unavailable"} else if diagnostics.is_empty() {"success"} else {"recovered"},
            "lowering_status":"not-run: dependency-closed library grammar unavailable",
            "unresolved_references":null,"ambiguous_references":null,
            "reference_status":"not-evaluated; null is not zero",
            "semantic_validation_status":"incomplete",
            "unsupported_semantic_evaluation":"All library evaluation unavailable; no function or execution semantics accepted",
            "element_count":0,"relationship_count":0,
            "count_scope":"No canonical library model was constructed",
            "diagnostics_by_category":{"syntax":if sysml {1} else {diagnostics.len()},"resolution":null,"kerml_semantic_validation":null,"sysml_semantic_validation":null,"strict_metamodel_conformance":"separate-runtime-command"},
        }));
    }
    documents.sort_by_key(|v| v["file"].as_str().unwrap().to_owned());
    quality.sort_by_key(|v| v["file"].as_str().unwrap().to_owned());
    let entries: Vec<_> = constructs.into_iter().map(|(construct,occurrences)| {
        let mut files = BTreeMap::<String,Vec<String>>::new();
        for o in &occurrences {
            files.entry(o["file"].as_str().unwrap().into()).or_default().push(format!("{}..{}",o["byte_range"][0],o["byte_range"][1]));
        }
        let ubiquitous = matches!(construct.as_str(),"trivia/whitespace"|"name/basic");
        let files: BTreeMap<_,_> = files.into_iter().map(|(file,ranges)| (file,json!({"count":ranges.len(),"byte_ranges":if ubiquitous {None} else {Some(ranges)}}))).collect();
        let supported = ["keyword/namespace","keyword/type","keyword/feature","keyword/abstract","keyword/specializes","keyword/typed","keyword/by","keyword/subsets","keyword/redefines"].contains(&construct.as_str());
        json!({"construct":construct,"evidence_kind":"lexical-occurrence; production classification still required",
            "count":occurrences.len(),"files":files,
            "range_encoding":"UTF-8 half-open start..end; ubiquitous whitespace/basic-name positions omitted (counts retained)",
            "parser_support":if supported {"bounded authored KerML slice only"} else {"see exact per-document probe diagnostics"},
            "lowering_support":if supported {"bounded authored KerML slice only"} else {"not-established"},
            "semantic_interpretation_support":"library semantics not implemented",
            "retained_but_not_evaluated":true})
    }).collect();
    let inventory = json!({
        "format":"agentique-library-syntax-coverage/1","library_set":libraries.content_set_id(),
        "status":"complete-byte-and-lexical-scan; production-inventory-incomplete",
        "scope":"All 57 exact pinned textual entries scanned. Tokens and recovery diagnostics are evidence, not a production-level parser or semantic library ingestion.",
        "required_production_families":["packages/namespaces","imports (membership/namespace)","aliases","visibility","short names","definitions/usages","specialization","subsetting","redefinition","conjugation","multiplicities","feature directions","value assignments","expressions","metadata","comments/annotations","classifiers","connectors","behaviors","functions"],
        "documents":documents,"constructs":entries,
    });
    let report = json!({"format":"agentique-standard-library-quality/1","library_set":libraries.content_set_id(),
        "result":"incomplete","semantic_quality_gate_passed":false,"documents":quality});
    let args: Vec<_> = std::env::args().skip(1).collect();
    let write = args.iter().any(|a| a == "--write");
    for (path, value) in [
        ("standards/library-syntax-coverage.json", inventory),
        (
            "verification/sysml-semantic-foundation-v1/library-quality.json",
            report,
        ),
    ] {
        let bytes = format!("{}\n", serde_json::to_string_pretty(&value)?);
        if write {
            std::fs::write(root.join(path), bytes)?;
        } else if std::fs::read(root.join(path))? != bytes.as_bytes() {
            return Err(format!("stale corpus report {path}").into());
        }
    }
    println!(
        "Verified 57 lossless source partitions and reproducible lexical inventory; semantic quality gate INCOMPLETE"
    );
    if args.iter().any(|a| a == "--require-semantic") {
        std::process::exit(1);
    }
    Ok(())
}

// Token classification vocabulary, never a parser or a model declaration extractor.
const KEYWORDS: &str = "about abstract alias all and as assoc behavior binding bool by chains class classifier comment composite conjugate conjugates conjugation connector const crosses datatype default dependency derived differences disjoining disjoint doc else end expr false feature featured featuring filter first flow for from function hastype if implies import in inout interaction intersects inv inverse inverting istype language library locale member meta metaclass metadata multiplicity namespace nonunique not null of or ordered out package portion predicate private protected public redefines redefinition references rep return specialization specializes standard step struct subclassifier subset subsets subtype succession then to true type typed typing unions var xor action assert assign attribute calc case concern connect connection constraint def entry event exhibit exit individual interface item part perform port ref rendering require requirement satisfy send snapshot state timeslice transition use variant variation verification verify view viewpoint when while";
