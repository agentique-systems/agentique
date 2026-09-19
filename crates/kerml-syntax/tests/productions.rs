use agq_kerml_syntax::{
    DocumentId, SourceRevisionId,
    production::{self, Document, Limits, Production as P},
};

fn parsed(source: &str) -> Document {
    production::parse(
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Limits::default(),
    )
    .unwrap()
}

#[test]
fn contextual_production_families_are_lossless_and_inspectable() {
    let fixtures = [
        (
            "standard library package <p> P { private import Q::*; public alias <x> a for Q::b; }",
            vec![
                P::LibraryPackage,
                P::NamespaceImport,
                P::AliasMember,
                P::VisibilityIndicator,
            ],
        ),
        (
            "abstract class C :> B { in feature <x> f : T [0..*] ordered nonunique :>> B::f; }",
            vec![
                P::Class,
                P::SuperclassingPart,
                P::FeatureDirection,
                P::MultiplicityBounds,
                P::Redefinitions,
            ],
        ),
        (
            "feature a chains b.c; feature d inverse of e; feature x ~ y;",
            vec![P::FeatureChain, P::InvertingPart, P::ConjugationPart],
        ),
        (
            "doc /* import false; */ comment c about P /* λ */",
            vec![P::Documentation, P::Comment, P::Annotation],
        ),
        (
            "metaclass M; metadata m : M { feature x = 1; }",
            vec![
                P::Metaclass,
                P::MetadataFeature,
                P::MetadataBodyFeature,
                P::LiteralInteger,
            ],
        ),
        (
            "function F { in a : T; return result : T; if a ? 1.2e3 else 0 }",
            vec![
                P::Function,
                P::ReturnFeatureMember,
                P::ResultExpressionMember,
                P::ConditionalExpression,
                P::LiteralReal,
            ],
        ),
        (
            "behavior B { step a; step b; succession first a then b; connector from a to b; binding a = b; }",
            vec![
                P::Behavior,
                P::Step,
                P::Succession,
                P::Connector,
                P::BindingConnector,
            ],
        ),
        (
            "interaction I { flow transfer from a.x to b.y; }",
            vec![P::Interaction, P::Flow, P::FlowEnd],
        ),
    ];
    for (source, expected) in fixtures {
        let doc = parsed(source);
        assert!(doc.is_complete(), "{source}: {:?}", doc.diagnostics());
        for kind in expected {
            assert!(doc.nodes().any(|n| n.kind() == kind), "{kind:?}: {source}");
        }
        let mut end = 0;
        for token in doc.tokens() {
            assert_eq!(token.range.start(), end);
            end = token.range.end();
            assert!(doc.source().is_char_boundary(end as usize));
        }
        assert_eq!(end as usize, source.len());
        assert_eq!(
            doc.tokens()
                .iter()
                .map(|t| doc.token_text(t))
                .collect::<String>(),
            source
        );
        assert!(doc.nodes().all(|n| doc.text(n.range()).is_some()));
    }
}

#[test]
fn expression_precedence_and_associativity_are_structural() {
    let doc = parsed("feature x = a + b * c ^ d ^ e;");
    assert!(doc.is_complete());
    assert_eq!(
        doc.nodes()
            .filter(|n| n.kind() == P::BinaryOperatorExpression)
            .map(|n| n.text())
            .collect::<Vec<_>>(),
        ["a + b * c ^ d ^ e", "b * c ^ d ^ e", "c ^ d ^ e", "d ^ e"]
    );
    let doc = parsed("feature x = a - b - c;");
    assert_eq!(
        doc.nodes()
            .filter(|n| n.kind() == P::BinaryOperatorExpression)
            .map(|n| n.text())
            .collect::<Vec<_>>(),
        ["a - b - c", "a - b"]
    );
    let doc = parsed("feature x = a or b and not c == d;");
    assert!(doc.is_complete());
    assert_eq!(
        doc.nodes()
            .find(|n| n.kind() == P::UnaryOperatorExpression)
            .unwrap()
            .text(),
        "not c"
    );
}

#[test]
fn incomplete_authored_input_recovers_without_hoisting_or_source_loss() {
    for source in [
        "feature good; package Broken { feature nested;",
        "private import ;",
        "feature bad = a + ;",
        "part def S { feature injected; }",
    ] {
        let doc = parsed(source);
        assert!(!doc.is_complete());
        assert!(!doc.recovery().is_empty());
        assert_eq!(doc.source(), source);
        assert!(
            !doc.nodes()
                .any(|n| n.kind() == P::FeatureIdentification && n.text() == "injected")
        );
    }
    let doc = parsed("feature good; package Broken {");
    assert!(
        doc.nodes()
            .any(|n| n.kind() == P::FeatureIdentification && n.text() == "good")
    );
}

#[test]
fn grammar_discrepancies_do_not_become_unknown_syntax_or_fabricated_names() {
    let doc = parsed("inv { true }");
    assert!(doc.is_complete());
    assert_eq!(doc.discrepancies().len(), 1);
    assert_eq!(doc.discrepancies()[0].code, "KG_ANONYMOUS_INVARIANT");
    assert!(!doc.nodes().any(|n| n.kind() == P::FeatureIdentification));
}

#[test]
fn parse_identity_is_deterministic_within_an_exact_source_revision_and_work_is_bounded() {
    let document = DocumentId::new();
    let revision = SourceRevisionId::new();
    let a = production::parse(document, revision, "feature x;", Limits::default()).unwrap();
    let b = production::parse(document, revision, "feature x;", Limits::default()).unwrap();
    assert_eq!(
        a.nodes()
            .map(|n| (n.id(), n.kind(), n.range()))
            .collect::<Vec<_>>(),
        b.nodes()
            .map(|n| (n.id(), n.kind(), n.range()))
            .collect::<Vec<_>>()
    );
    assert!(
        production::parse(
            document,
            revision,
            "feature x;",
            Limits {
                max_chart_items: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
}
