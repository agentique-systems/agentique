//! The Scenario A model: it loads, validates, round-trips and survives edits.
use agq_language::{ElementKind, Source, Tree, parse, print, validate};

const MODEL: &str = include_str!("../../../models/url-shortener/UrlShortener.sysml");
const BROKEN: &str = include_str!("fixtures/UrlShortenerBroken.sysml");

fn load(text: &str) -> Tree {
    parse(&[Source::new("UrlShortener.sysml", text)])
}

#[test]
fn the_model_is_valid() {
    let tree = load(MODEL);
    assert_eq!(validate(&tree), []);
    assert!(
        tree.find("UrlShortener::UrlShortenerService::clickReporting")
            .is_some()
    );
}

#[test]
fn print_then_parse_gives_the_same_tree() {
    let tree = load(MODEL);
    let printed = print(&tree);
    let reparsed = parse(&printed);
    assert_eq!(
        tree.clone().without_locations(),
        reparsed.clone().without_locations()
    );
    // Printing is canonical: printing the reparsed tree changes nothing.
    assert_eq!(print(&reparsed), printed);
    // `doc` comments survive, `//` notes do not.
    let text = &printed[0].text;
    assert!(text.contains("doc /* Persists short links and looks them up by code. */"));
    assert!(!text.contains("// Ports."));
}

#[test]
fn the_deliberate_error_is_reported_at_the_connection() {
    let tree = load(BROKEN);
    let diagnostics = validate(&tree);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let d = &diagnostics[0];
    assert_eq!(d.code, "incompatible-ends");
    assert_eq!(
        tree.qualified_name(d.element),
        "UrlShortener::UrlShortenerService::clickReporting"
    );
    let line = BROKEN
        .lines()
        .position(|l| l.contains("interface clickReporting"))
        .unwrap()
        + 1;
    assert_eq!(d.location.unwrap().line as usize, line);
    assert!(
        d.message.contains("`clickStats.clicks` (ClickPort)"),
        "{}",
        d.message
    );
    assert!(d.message.contains("`sink` (~ClickPort)"), "{}", d.message);
}

#[test]
fn an_edit_keeps_identities_and_is_reported_where_it_breaks_references() {
    let mut tree = load(MODEL);
    let api = tree.find("UrlShortener::UrlShortenerService::api").unwrap();
    let ids_before = tree.walk();
    tree.get_mut(api).name = Some("gateway".into());
    assert_eq!(tree.walk(), ids_before);

    let diagnostics = validate(&tree);
    let broken: Vec<(String, &str)> = diagnostics
        .iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect();
    assert_eq!(
        broken,
        [
            (
                "UrlShortener::UrlShortenerService::storage".to_string(),
                "unresolved"
            ),
            (
                "UrlShortener::UrlShortenerService::clickReporting".to_string(),
                "unresolved"
            ),
            (
                "UrlShortener::UrlShortenerService::statsQuery".to_string(),
                "unresolved"
            ),
            (
                "UrlShortener::(satisfy fastRedirect)".to_string(),
                "unresolved"
            ),
        ]
    );
    assert_eq!(
        diagnostics[0].message,
        "cannot find `api` in the connection end `api.storage`"
    );

    tree.get_mut(api).name = Some("api".into());
    assert_eq!(validate(&tree), []);
}

#[test]
fn removing_a_definition_reports_its_users() {
    let mut tree = load(MODEL);
    let link_store = tree.find("UrlShortener::LinkStore").unwrap();
    let removed = tree.remove(link_store);
    assert_eq!(removed.len(), 4); // the part def, its doc and its two features
    let codes: Vec<(String, &str)> = validate(&tree)
        .iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect();
    assert!(codes.contains(&("UrlShortener::SqlLinkStore".into(), "unresolved")));
    assert!(codes.contains(&("UrlShortener::UniqueCodes::store".into(), "unresolved")));
    assert!(
        tree.walk()
            .iter()
            .all(|id| tree[*id].kind != ElementKind::SyntaxError)
    );
}
