//! Source inventory over verified bytes and the existing lossless lexer.
//!
//! This is deliberately not a SysML grammar recognizer or a semantic loader.
//! Token surfaces identify the corpus-driven work; counts do not certify parsing.
use agq_kerml_syntax::{ParseLimits, TokenKind};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

const FAMILIES: &[(&str, &[&str])] = &[
    ("attribute", &["attribute"]),
    ("part", &["part"]),
    ("item", &["item"]),
    ("port", &["port"]),
    ("connection", &["connection"]),
    ("interface", &["interface"]),
    ("flow", &["flow"]),
    ("message", &["message"]),
    ("action", &["action"]),
    ("state", &["state"]),
    ("requirement", &["requirement"]),
    ("constraint", &["constraint"]),
    ("calculation", &["calc"]),
    ("analysis_case", &["analysis"]),
    ("verification_case", &["verification"]),
    ("use_case", &["use", "case"]),
    ("case", &["case"]),
    ("metadata", &["metadata"]),
    ("enumeration", &["enum"]),
    ("occurrence", &["occurrence"]),
    ("allocation", &["allocation"]),
    ("concern", &["concern"]),
    ("view", &["view"]),
    ("viewpoint", &["viewpoint"]),
    ("rendering", &["rendering"]),
];

const SHARED_SURFACES: &[(&str, &[&str])] = &[
    ("Package", &["package"]),
    ("Documentation", &["doc"]),
    ("Import", &["import"]),
    ("AliasMember", &["alias"]),
    ("SpecializesSymbol", &[":>"]),
    ("SpecializesKeyword", &["specializes"]),
    ("SubsetsKeyword", &["subsets"]),
    ("RedefinesSymbol", &[":>>"]),
    ("RedefinesKeyword", &["redefines"]),
    ("MultiplicityOpen", &["["]),
    ("QualifiedNameSeparator", &["::"]),
    ("FeatureChainSeparator", &["."]),
    ("FeatureValueEquals", &["="]),
    ("FeatureValueDefault", &["default"]),
    ("ArrowExpression", &["->"]),
    ("IndexExpression", &["#", "("]),
    ("AsExpression", &["as"]),
    ("ConditionalExpression", &["if"]),
    ("ReturnParameter", &["return"]),
    ("BindingConnector", &["bind"]),
];

const EXTENSION_SURFACES: &[(&str, &[&str])] = &[
    ("ReferenceUsage", &["ref"]),
    ("SubjectMember", &["subject"]),
    ("ObjectiveMember", &["objective"]),
    ("RequireConstraintMember", &["require"]),
    ("AssumeConstraintMember", &["assume"]),
    ("SatisfyRequirementUsage", &["satisfy"]),
    ("AssertConstraintUsage", &["assert"]),
    ("EnumerationDefinition", &["enum", "def"]),
    ("EnumerationUsage", &["enum"]),
    ("EventOccurrenceUsage", &["event", "occurrence"]),
    ("ConnectionUsageConnect", &["connect"]),
    ("SuccessionAsUsage", &["succession"]),
    ("AssignmentActionUsage", &["assign"]),
    ("PerformActionUsage", &["perform"]),
    ("WhileLoopActionUsage", &["while"]),
    ("ActionSuccession", &["then"]),
    ("StateEntryMember", &["entry"]),
    ("StateDoMember", &["do"]),
    ("StateExitMember", &["exit"]),
    ("TransitionUsage", &["transition"]),
    ("ConjugatedPortTyping", &["~"]),
];

#[derive(Clone, Copy)]
struct Token<'a> {
    kind: TokenKind,
    text: &'a str,
    start: u64,
    end: u64,
}

fn matches(tokens: &[Token<'_>], at: usize, pattern: &[&str]) -> bool {
    tokens.get(at..at + pattern.len()).is_some_and(|window| {
        window.iter().zip(pattern).all(|(token, expected)| {
            matches!(token.kind, TokenKind::Word | TokenKind::Symbol) && token.text == *expected
        })
    })
}

fn surfaces(tokens: &[Token<'_>], patterns: &[(&str, &[&str])]) -> Value {
    let mut out = BTreeMap::new();
    for (name, pattern) in patterns {
        let positions: Vec<_> = (0..tokens.len())
            .filter(|&at| matches(tokens, at, pattern))
            .collect();
        if let Some(&first) = positions.first() {
            out.insert(
                name,
                json!({"occurrences": positions.len(), "first_byte_range": [tokens[first].start, tokens[first + pattern.len() - 1].end]}),
            );
        }
    }
    json!(out)
}

fn document(
    source: &agq_standard_libraries::LibraryDocument,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Use only the shared lexer output. The bounded KerML parser's recovery is
    // expected for SysML and cannot establish a SysML grammar outcome.
    let lexed =
        agq_kerml_syntax::parse(source.document(), source.source(), ParseLimits::default())?;
    assert!(lexed.diagnostics().iter().all(|d| d.code != "KS_LEXICAL"));
    let mut end = 0;
    for token in lexed.tokens() {
        assert_eq!(token.range.start(), end);
        end = token.range.end();
    }
    assert_eq!(end as usize, source.source().len());
    let round_trip: String = lexed.tokens().iter().map(|t| lexed.token_text(t)).collect();
    assert_eq!(round_trip.as_bytes(), source.source().as_bytes());
    let tokens: Vec<_> = lexed
        .tokens()
        .iter()
        .filter(|t| !t.kind.is_trivia() && t.kind != TokenKind::Comment)
        .map(|t| Token {
            kind: t.kind,
            text: lexed.token_text(t),
            start: t.range.start(),
            end: t.range.end(),
        })
        .collect();
    let mut imports = vec![];
    for (at, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Word || token.text != "import" {
            continue;
        }
        let end = (at + 1..tokens.len())
            .find(|&i| tokens[i].text == ";")
            .expect("import terminator");
        let visibility = at
            .checked_sub(1)
            .map(|i| tokens[i].text)
            .filter(|text| matches!(*text, "private" | "protected" | "public"))
            .unwrap_or("public");
        imports.push(json!({"target": tokens[at + 1..end].iter().map(|t| t.text).collect::<String>(), "visibility": visibility, "byte_range": [token.start, tokens[end].end]}));
    }
    let mut families = BTreeMap::new();
    let mut definitions = 0;
    let mut usages = 0;
    for (family, pattern) in FAMILIES {
        let positions: Vec<_> = (0..tokens.len())
            .filter(|&at| {
                matches(&tokens, at, pattern)
                    && !(pattern == &["case"]
                        && at > 0
                        && matches!(tokens[at - 1].text, "analysis" | "verification" | "use"))
            })
            .collect();
        let defs = positions
            .iter()
            .filter(|&&at| matches(&tokens, at + pattern.len(), &["def"]))
            .count();
        let uses = positions.len() - defs;
        definitions += defs;
        usages += uses;
        if defs != 0 || uses != 0 {
            families.insert(
                family,
                json!({"definition_introducers": defs, "usage_keyword_surfaces": uses}),
            );
        }
    }
    assert_eq!(
        definitions,
        tokens
            .iter()
            .filter(|t| t.kind == TokenKind::Word && t.text == "def")
            .count(),
        "every definition introducer must be inventoried"
    );
    let anonymous_references = tokens
        .iter()
        .enumerate()
        .filter(|(at, token)| {
            token.kind == TokenKind::Word
                && token.text == "ref"
                && !FAMILIES
                    .iter()
                    .any(|(_, pattern)| matches(&tokens, at + 1, pattern))
        })
        .count();
    Ok(json!({
        "path": source.path(), "bytes": source.source().len(), "sha256": source.sha256(),
        "library_id": source.library().to_string(), "document_id": source.document().to_string(), "source_revision_id": source.revision().to_string(),
        "imports": imports,
        "definition_introducers": definitions, "usage_family_keyword_surfaces": usages,
        "reference_usage_introducers_without_family_keyword": anonymous_references,
        "families": families,
        "shared_kerml_production_surfaces": surfaces(&tokens, SHARED_SURFACES),
        "sysml_extension_production_surfaces": surfaces(&tokens, EXTENSION_SURFACES),
        "lossless_lexing": {"tokens": lexed.tokens().len(), "lexical_diagnostics": 0, "exact_byte_preservation": true},
        "sysml_grammar_status": "not-evaluated-no-sysml-frontend", "canonical_lowering_status": "not-started"
    }))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let set = VerifiedLibrarySet::load_from_directory(&root)?;
    let library = set
        .libraries()
        .values()
        .find(|l| l.project().name == "SysML Systems Library")
        .expect("pinned Systems Library");
    let mut documents: Vec<_> = set
        .documents()
        .filter(|d| d.language() == LibraryLanguage::SysMl)
        .map(document)
        .collect::<Result<_, _>>()?;
    documents.sort_by_key(|d| d["path"].as_str().unwrap().to_owned());
    assert_eq!(documents.len(), 21);
    let report = json!({
        "format": "agentique-sysml-systems-source-inventory/1",
        "method": "Exact verified original KPAR bytes; shared agq-kerml-syntax lossless lexer; keyword/symbol surfaces excluding quoted names, strings, comments and notes. Production names identify required grammar work, not recognized productions or canonical records. A ref prefix on a family usage is not another Usage. Counts of enum, event, requirement and action surfaces can overlap specialized constructs. Omitted family/surface entries have zero occurrences. Zero means absent lexical evidence, not an implemented semantic rule.",
        "family_categories": FAMILIES.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "authority": {"specification": "SysML 2.0", "document": "formal/26-03-02", "metamodel_uri": library.metadata().metamodel},
        "library_set": set.content_set_id(), "library_id": library.id().to_string(), "resource": library.resource(), "archive_sha256": library.archive_sha256(),
        "project_version": library.project().version,
        "dependencies": library.project().usage.iter().map(|usage| json!({"resource": usage.resource, "version_constraint": usage.version_constraint})).collect::<Vec<_>>(),
        "metadata_findings": set.diagnostics().iter().map(|d| format!("{d:?}")).collect::<Vec<_>>(),
        "document_count": documents.len(),
        "source_bytes": documents.iter().map(|d| d["bytes"].as_u64().unwrap()).sum::<u64>(),
        "definition_introducers": documents.iter().map(|d| d["definition_introducers"].as_u64().unwrap()).sum::<u64>(),
        "usage_family_keyword_surfaces": documents.iter().map(|d| d["usage_family_keyword_surfaces"].as_u64().unwrap()).sum::<u64>(),
        "sysml_documents_parsed": 0, "sysml_semantics_started": false,
        "documents": documents
    });
    let bytes = serde_json::to_vec_pretty(&report)?;
    let path = root.join("verification/summaries/sysml-semantic-foundation/corpus-inventory.json");
    match std::env::args().nth(1).as_deref() {
        Some("--write") => {
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, [&bytes[..], b"\n"].concat())?;
        }
        Some("--check") => {
            assert_eq!(
                std::fs::read(path)?,
                [&bytes[..], b"\n"].concat(),
                "stale Systems Library source inventory"
            );
        }
        _ => return Err("use --write or --check".into()),
    }
    println!(
        "21 pinned Systems Library documents; {} source bytes; lossless lexer 21/21; SysML parse acceptance 0/21; {} definition introducers; {} usage-family keyword surfaces; no semantic publication claimed",
        report["source_bytes"],
        report["definition_introducers"],
        report["usage_family_keyword_surfaces"]
    );
    Ok(())
}
