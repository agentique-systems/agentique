//! Exact verified corpus tokens for the independent grammar-production inventory.
use agq_kerml_syntax::{ParseLimits, TokenKind};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use serde_json::json;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let set = VerifiedLibrarySet::load_from_directory(&root)?;
    let mut documents = vec![];
    for doc in set
        .documents()
        .filter(|d| d.language() == LibraryLanguage::KerMl)
    {
        let parsed = agq_kerml_syntax::parse(doc.document(), doc.source(), ParseLimits::default())?;
        let tokens: Vec<_> = parsed
            .tokens()
            .iter()
            .filter(|t| !t.kind.is_trivia())
            .map(|t| {
                let kind = match t.kind {
                    TokenKind::Word => "word",
                    TokenKind::QuotedName => "NAME",
                    TokenKind::DecimalValue => "DECIMAL_VALUE",
                    TokenKind::ExponentialValue => "EXPONENTIAL_VALUE",
                    TokenKind::StringValue => "STRING_VALUE",
                    TokenKind::Comment => "REGULAR_COMMENT",
                    _ => "symbol",
                };
                json!([kind, parsed.token_text(t), t.range.start(), t.range.end()])
            })
            .collect();
        documents.push(
            json!({"file":doc.path(),"sha256":doc.sha256(),"source":doc.source(),"tokens":tokens}),
        );
    }
    documents.sort_by_key(|d| d["file"].as_str().unwrap().to_owned());
    println!(
        "{}",
        serde_json::to_string(&json!({"library_set":set.content_set_id(),"documents":documents}))?
    );
    Ok(())
}
