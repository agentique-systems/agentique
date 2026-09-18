use agq_kerml_syntax::{DocumentId, ParseLimits, TokenKind, parse};

#[test]
fn normative_unsigned_numeric_tokens_preserve_literal_and_range_punctuation() {
    let source = "1..* -12 +3 1.25 .5 12e-7 1.2E+34 9E8 1e 2E+";
    let syntax = parse(DocumentId::new(), source, ParseLimits::default()).unwrap();
    assert!(!syntax.diagnostics().iter().any(|d| d.code == "KS_LEXICAL"));
    let actual: Vec<_> = syntax
        .tokens()
        .iter()
        .filter(|t| !t.kind.is_trivia())
        .map(|t| (syntax.token_text(t), t.kind))
        .collect();
    use TokenKind::{DecimalValue as D, ExponentialValue as E, Symbol as S, Word as W};
    assert_eq!(
        actual,
        vec![
            ("1", D),
            ("..", S),
            ("*", S),
            ("-", S),
            ("12", D),
            ("+", S),
            ("3", D),
            ("1", D),
            (".", S),
            ("25", D),
            (".", S),
            ("5", D),
            ("12e-7", E),
            ("1", D),
            (".", S),
            ("2E+34", E),
            ("9E8", E),
            ("1", D),
            ("e", W),
            ("2", D),
            ("E", W),
            ("+", S)
        ]
    );
    assert_eq!(
        syntax
            .tokens()
            .iter()
            .map(|t| syntax.token_text(t))
            .collect::<String>(),
        source
    );
    // Recognition of numeric tokens does not claim expression grammar/evaluation.
    assert!(!syntax.diagnostics().is_empty());
}
