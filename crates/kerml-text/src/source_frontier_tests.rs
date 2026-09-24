use super::*;
use syntax::production;

fn parsed(source: &str) -> production::Document {
    production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV3,
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Default::default(),
    )
    .unwrap()
}

fn exact_changed(
    before: &production::Document,
    after: &production::Document,
) -> BTreeSet<agq_kernel::SyntaxNodeId> {
    let old: BTreeMap<_, _> = before.nodes().map(|node| (node.id(), node)).collect();
    after
        .nodes()
        .filter_map(|node| {
            let prior = old.get(&node.id())?;
            (prior.kind() != node.kind()
                || prior.range() != node.range()
                || prior.text() != node.text()
                || !prior
                    .children()
                    .map(|child| child.id())
                    .eq(node.children().map(|child| child.id())))
            .then_some(node.id())
        })
        .collect()
}

#[test]
fn retained_syntax_frontier_reports_exact_content_ranges_and_children() {
    let before = parsed("package P { part def A; part def B; }");
    assert!(before.is_complete());
    assert!(changed_document_syntax(&before, &before).is_empty());

    // A carrier can deliberately preserve an identity while same-width content
    // changes. Production shape remains identical; source content must still be compared.
    let changed = parsed("package P { part def C; part def B; }")
        .restore_identities(&before.identity_checkpoint())
        .unwrap();
    let actual = changed_document_syntax(&before, &changed);
    assert_eq!(actual, exact_changed(&before, &changed));
    assert!(!actual.is_empty());
    assert!(
        before
            .nodes()
            .filter(|node| node.text() == "B")
            .all(|node| !actual.contains(&node.id()))
    );

    // Identical bytes with a replaced child identity must change its retained parent.
    let mut identities = before.identity_checkpoint();
    let child = identities
        .iter()
        .flat_map(|node| node.children.iter())
        .copied()
        .find(|index| identities[*index].children.is_empty())
        .unwrap();
    identities[child].id = agq_kernel::SyntaxNodeId::new();
    let replaced = parsed(before.source())
        .restore_identities(&identities)
        .unwrap();
    let actual = changed_document_syntax(&before, &replaced);
    assert_eq!(actual, exact_changed(&before, &replaced));
    assert!(!actual.is_empty());

    // Ordinary edit reconciliation retains following declarations but shifts their ranges.
    let offset = before.source().find("A;").unwrap() as u64;
    let shifted = before
        .edit(
            &syntax::TextEdit {
                range: agq_kernel::provenance::ByteRange::new(offset, offset + 1).unwrap(),
                replacement: "Longer".into(),
            },
            Default::default(),
        )
        .unwrap();
    let actual = changed_document_syntax(&before, &shifted);
    assert_eq!(actual, exact_changed(&before, &shifted));
    let old: BTreeMap<_, _> = before
        .nodes()
        .map(|node| (node.id(), node.range()))
        .collect();
    assert!(shifted.nodes().any(|node| {
        old.get(&node.id())
            .is_some_and(|range| *range != node.range())
            && actual.contains(&node.id())
    }));
}
