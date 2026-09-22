use agq_kerml_syntax::{
    ByteRange, DocumentId, SourceError, SourceRevisionId, TextEdit,
    production::{self, Dialect, Document, Limits, Production as P, SysmlSyntaxProfile},
};
use agq_standard_libraries::{LibraryLanguage, VerifiedLibrarySet};
use std::path::Path;

fn parsed(source: &str) -> Document {
    production::parse_sysml(
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Limits::default(),
    )
    .unwrap()
}

fn operational(source: &str) -> Document {
    production::parse_sysml_with_profile(
        SysmlSyntaxProfile::OperationalV1,
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Limits::default(),
    )
    .unwrap()
}

#[test]
fn operational_v2_reuses_v1_grammar_with_a_distinct_profile_identity() {
    let source = "part def Engine;";
    let v1 = operational(source);
    let v2 = production::parse_sysml_with_profile(
        SysmlSyntaxProfile::OperationalV2,
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Limits::default(),
    )
    .unwrap();
    assert!(v2.is_complete(), "{:?}", v2.diagnostics());
    assert_lossless(&v2, source);
    assert_eq!(
        SysmlSyntaxProfile::OperationalV1.grammar_compatibility_manifest_sha256(),
        SysmlSyntaxProfile::OperationalV2.grammar_compatibility_manifest_sha256()
    );
    assert_ne!(
        SysmlSyntaxProfile::OperationalV1.id(),
        SysmlSyntaxProfile::OperationalV2.id()
    );
    assert_eq!(v1.nodes().count(), v2.nodes().count());
}

fn assert_lossless(doc: &Document, source: &str) {
    assert_eq!(doc.source(), source);
    let mut end = 0;
    for token in doc.tokens() {
        assert_eq!(token.range.start(), end);
        end = token.range.end();
        assert!(source.is_char_boundary(end as usize));
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

#[test]
fn part_vertical_keeps_shared_relationship_productions_and_origins() {
    let source = "part def Engine;\npart def Vehicle { part engine : Engine; }\npart def SportsCar :> Vehicle;";
    let doc = parsed(source);
    assert!(doc.is_complete(), "{:?}", doc.diagnostics());
    assert_eq!(doc.dialect(), Dialect::SysMl);
    assert_lossless(&doc, source);
    assert_eq!(
        doc.nodes()
            .filter(|n| n.kind() == P::PartDefinition)
            .count(),
        3
    );
    assert_eq!(doc.nodes().filter(|n| n.kind() == P::PartUsage).count(), 1);
    for kind in [
        P::PackageMember,
        P::OccurrenceUsageMember,
        P::OwnedFeatureTyping,
        P::OwnedSubclassification,
    ] {
        assert!(doc.nodes().any(|n| n.kind() == kind), "{kind:?}");
    }
    let typing = doc
        .nodes()
        .find(|n| n.kind() == P::OwnedFeatureTyping)
        .unwrap();
    assert_eq!(typing.text(), "Engine");
    assert_eq!(typing.origin().document, doc.document());
    assert_eq!(typing.origin().revision, doc.revision());
    assert_eq!(typing.origin().syntax_node, Some(typing.id()));
    let sub = doc
        .nodes()
        .find(|n| n.kind() == P::OwnedSubclassification)
        .unwrap();
    assert_eq!(sub.text(), "Vehicle");
}

#[test]
fn sysml_edits_preserve_dialect_and_unaffected_node_identity() {
    let doc = parsed("part def Engine; part def Vehicle;");
    let engine = doc.nodes().find(|n| n.kind() == P::PartDefinition).unwrap();
    let first_id = engine.id();
    let start = doc.source().find("Vehicle").unwrap();
    let edited = doc
        .edit(
            &TextEdit {
                range: ByteRange::new(start as u64, (start + 8) as u64).unwrap(),
                replacement: "Carrier { part power : Engine; }".into(),
            },
            Limits::default(),
        )
        .unwrap();
    assert!(edited.is_complete(), "{:?}", edited.diagnostics());
    assert_eq!(edited.dialect(), Dialect::SysMl);
    assert_eq!(edited.document(), doc.document());
    assert_ne!(edited.revision(), doc.revision());
    assert_eq!(
        edited
            .nodes()
            .find(|n| n.kind() == P::PartDefinition)
            .unwrap()
            .id(),
        first_id
    );
    assert_lossless(&edited, edited.source());
}

#[test]
fn dialects_share_lexing_but_keep_their_reserved_names_and_identity_domain() {
    let sysml = parsed("part def behavior;");
    assert!(sysml.is_complete(), "{:?}", sysml.diagnostics());
    assert!(
        sysml.nodes().any(
            |n| n.kind() == P::Identification && n.names().any(|name| name.value == "behavior")
        )
    );
    assert!(!parsed("part def part;").is_complete());
    assert!(parsed("part def 'part';").is_complete());
    let id = DocumentId::new();
    let revision = SourceRevisionId::new();
    let kerml = production::parse(id, revision, "package P;", Limits::default()).unwrap();
    let sysml = production::parse_sysml(id, revision, "package P;", Limits::default()).unwrap();
    assert_eq!(kerml.tokens(), sysml.tokens());
    assert_ne!(
        kerml.roots().next().unwrap().id(),
        sysml.roots().next().unwrap().id()
    );
    assert!(
        !production::parse(id, revision, "part def Engine;", Limits::default())
            .unwrap()
            .is_complete()
    );
}

#[test]
fn shared_expression_precedence_and_empty_port_wrappers_are_retained() {
    let doc = parsed("attribute amount = a + b * c ^ d ^ e; port def Socket;");
    assert!(doc.is_complete(), "{:?}", doc.diagnostics());
    assert_eq!(
        doc.nodes()
            .filter(|n| n.kind() == P::BinaryOperatorExpression)
            .map(|n| n.text())
            .collect::<Vec<_>>(),
        ["a + b * c ^ d ^ e", "b * c ^ d ^ e", "c ^ d ^ e", "d ^ e"]
    );
    let port = doc.nodes().find(|n| n.kind() == P::PortDefinition).unwrap();
    for kind in [
        P::ConjugatedPortDefinitionMember,
        P::ConjugatedPortDefinition,
        P::PortConjugation,
    ] {
        let node = port.descendants().find(|n| n.kind() == kind).unwrap();
        assert_eq!(node.range().start(), node.range().end());
        assert!(node.origin().syntax_node.is_some());
    }
}

#[test]
fn published_grammar_retains_reviewed_compatibility_failures() {
    for source in [
        "allocation def Allocation;",
        "case def Case { return result : Result; }",
        "connection def Link { end item source; }",
        "connection def Link { end source; }",
        "satisfy requirement check;",
    ] {
        let doc = parsed(source);
        assert!(
            !doc.is_complete(),
            "Unexpectedly adopted grammar interpretation: {source}"
        );
        assert_lossless(&doc, source);
        assert!(doc.diagnostics().iter().any(|d| d.code == "SG_RECOVERY"));
    }
    assert!(parsed("assert not satisfy requirement check;").is_complete());
}

#[test]
fn malformed_sysml_keeps_source_without_hoisting_nested_declarations() {
    let source = "part def Broken { part nested : Engine;";
    let doc = parsed(source);
    assert!(!doc.is_complete());
    assert_lossless(&doc, source);
    assert!(!doc.nodes().any(|n| n.kind() == P::PartUsage));
    let result = production::parse_sysml(
        DocumentId::new(),
        SourceRevisionId::new(),
        "part def P;",
        Limits {
            max_chart_items: 1,
            ..Limits::default()
        },
    );
    assert!(matches!(result, Err(SourceError::Limit("grammar chart"))));
}

#[test]
fn all_pinned_systems_documents_are_lossless_and_strict_outcomes_are_explicit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let libraries = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let mut complete = Vec::new();
    let mut incomplete = Vec::new();
    let mut operational_complete = 0;
    for source in libraries
        .documents()
        .filter(|d| d.language() == LibraryLanguage::SysMl)
    {
        let doc = production::parse_sysml(
            source.document(),
            source.revision(),
            source.source(),
            Limits::default(),
        )
        .unwrap();
        assert_lossless(&doc, source.source());
        assert_eq!(doc.document(), source.document());
        assert_eq!(doc.revision(), source.revision());
        let operational = production::parse_sysml_with_profile(
            SysmlSyntaxProfile::OperationalV1,
            source.document(),
            source.revision(),
            source.source(),
            Limits::default(),
        )
        .unwrap();
        assert_lossless(&operational, source.source());
        assert!(
            operational.is_complete(),
            "{}: {:?}",
            source.path(),
            operational.diagnostics()
        );
        assert_eq!(operational.recovery().len(), 0);
        assert_eq!(operational.document(), source.document());
        assert_eq!(operational.revision(), source.revision());
        assert_eq!(
            operational
                .roots()
                .map(|node| node.kind())
                .collect::<Vec<_>>(),
            [P::RootNamespace]
        );
        operational_complete += 1;
        if doc.is_complete() {
            complete.push(source.path().to_owned());
        } else {
            let [diagnostic] = doc.diagnostics() else {
                panic!("Changed historical diagnostics: {}", source.path());
            };
            assert_eq!(diagnostic.code, "SG_RECOVERY");
            assert_eq!(doc.recovery().len(), 1);
            incomplete.push((
                source.path().to_owned(),
                diagnostic.range.start(),
                diagnostic.range.end(),
                doc.text(diagnostic.range).unwrap().to_owned(),
            ));
        }
    }
    complete.sort();
    incomplete.sort();
    assert_eq!(complete.len() + incomplete.len(), 21);
    assert_eq!(operational_complete, 21);
    assert_eq!(
        complete.len(),
        13,
        "Unexpected strict accepted population; incomplete={incomplete:?}"
    );
    assert_eq!(
        incomplete,
        [
            (
                "Systems Library/Allocations.sysml".to_owned(),
                246,
                249,
                "def".to_owned()
            ),
            (
                "Systems Library/Cases.sysml".to_owned(),
                1183,
                1189,
                "return".to_owned()
            ),
            (
                "Systems Library/Connections.sysml".to_owned(),
                1630,
                1631,
                ";".to_owned()
            ),
            (
                "Systems Library/Flows.sysml".to_owned(),
                1476,
                1486,
                "occurrence".to_owned()
            ),
            (
                "Systems Library/Interfaces.sysml".to_owned(),
                2934,
                2938,
                "port".to_owned()
            ),
            (
                "Systems Library/Items.sysml".to_owned(),
                3681,
                3685,
                "item".to_owned()
            ),
            (
                "Systems Library/VerificationCases.sysml".to_owned(),
                669,
                675,
                "return".to_owned()
            ),
            (
                "Systems Library/Views.sysml".to_owned(),
                846,
                853,
                "satisfy".to_owned()
            ),
        ]
    );
}

#[test]
fn operational_grammar_accepts_only_the_reviewed_compatibility_shapes() {
    for (source, kind) in [
        ("allocation def Allocation;", P::AllocationDefinition),
        (
            "case def Case { return result : Result; }",
            P::ReturnParameterMember,
        ),
        ("connection def Link { end item source; }", P::ItemUsage),
        (
            "connection def Link { end source; }",
            P::DefaultReferenceUsage,
        ),
        (
            "flow def Transfer { end occurrence source; }",
            P::OccurrenceUsage,
        ),
        ("interface def Connector { end port source; }", P::PortUsage),
        (
            "item def Item { end touchesToo [0..*] item touchedItemToo; }",
            P::OwnedCrossFeatureMember,
        ),
        (
            "satisfy requirement check by that;",
            P::SatisfyRequirementUsage,
        ),
        (
            "assert satisfy requirement check;",
            P::SatisfyRequirementUsage,
        ),
        ("not satisfy requirement check;", P::SatisfyRequirementUsage),
        (
            "assert not satisfy requirement check;",
            P::SatisfyRequirementUsage,
        ),
    ] {
        let doc = operational(source);
        assert!(doc.is_complete(), "{source}: {:?}", doc.diagnostics());
        assert_lossless(&doc, source);
        assert!(
            doc.nodes().any(|node| node.kind() == kind),
            "{source}: {kind:?}"
        );
    }
    for source in [
        "end allocation def Allocation;",
        "case def Case { return return; }",
        "end end item duplicate;",
        "end individual sample;",
        "requirement check by that;",
        "satisfy by that;",
    ] {
        let doc = operational(source);
        assert!(!doc.is_complete(), "Overbroad compatibility: {source}");
        assert_lossless(&doc, source);
    }
}

#[test]
fn profile_and_identity_survive_edits_without_changing_historical_identity() {
    let id = DocumentId::new();
    let revision = SourceRevisionId::new();
    let strict = production::parse_sysml(id, revision, "package P;", Limits::default()).unwrap();
    let op = production::parse_sysml_with_profile(
        SysmlSyntaxProfile::OperationalV1,
        id,
        revision,
        "package P;",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(strict.sysml_profile(), Some(SysmlSyntaxProfile::Published));
    assert_eq!(op.sysml_profile(), Some(SysmlSyntaxProfile::OperationalV1));
    assert_ne!(
        strict.roots().next().unwrap().id(),
        op.roots().next().unwrap().id()
    );
    assert_eq!(strict.tokens(), op.tokens());
    let op = operational("allocation def Allocation;");
    let edited = op
        .edit(
            &TextEdit {
                range: ByteRange::new(15, 25).unwrap(),
                replacement: "Replacement".into(),
            },
            Limits::default(),
        )
        .unwrap();
    assert!(edited.is_complete(), "{:?}", edited.diagnostics());
    assert_eq!(edited.sysml_profile(), op.sysml_profile());
    assert_eq!(
        SysmlSyntaxProfile::Published.grammar_compatibility_manifest_sha256(),
        None
    );
    assert_eq!(
        SysmlSyntaxProfile::OperationalV1
            .grammar_compatibility_manifest_sha256()
            .unwrap()
            .len(),
        64
    );
}
