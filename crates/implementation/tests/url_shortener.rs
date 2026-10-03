//! The proof of Scenario I against real code (C-50, W8.4): the URL
//! shortener with AI screening, implemented from its model in a repository
//! of its own (a fixture copied into a temporary git repository), linked to
//! the model, checked, and run against the same scenarios as the model.
//! Then the implementation is broken the way the scenarios guard against (a
//! held link goes live), the break is found at the elements it concerns,
//! and repaired; and a boundary the model does not allow is found too.
use agq_execution::{Executor, Scope, git};
use agq_implementation::checks::{contract_shapes, linked_tests, module_boundaries};
use agq_implementation::{LinkKind, Links, drift, run_implementation};
use agq_language::{ElementKind, Source, Tree, parse};
use agq_simulation::digest::model_digest;
use agq_simulation::{Mode, Request, RunStatus, Verdict, compile};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const MODEL: &str = include_str!("../../../models/link-screening/UrlShortener.sysml");

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/url-shortener")
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "target" || name == "Cargo.lock" {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy(&path, &to.join(name));
        } else {
            std::fs::copy(&path, to.join(name)).unwrap();
        }
    }
}

/// The links a person (or a worker) would keep in `model/links.json`: the
/// fixture's table, by name.
pub fn links(tree: &Tree) -> Links {
    const TABLE: &[(&str, &str, &str, Option<&str>)] = include!("fixtures/url-shortener/links.in");
    let mut links = Links {
        language: "Rust".into(),
        harness: [
            "cargo",
            "run",
            "--quiet",
            "--offline",
            "--bin",
            "agentique-harness",
        ]
        .map(String::from)
        .to_vec(),
        protected: vec![
            "tests/behaviour.rs".into(),
            "tests/jev_client.rs".into(),
            "tests/decisions.json".into(),
        ],
        ..Links::default()
    };
    for (element, kind, path, symbol) in TABLE {
        let id = tree.find(&format!("UrlShortener::{element}")).unwrap();
        let kind = LinkKind::ALL
            .into_iter()
            .find(|k| k.key() == *kind)
            .unwrap();
        links.add(tree, id, kind, path, *symbol);
    }
    links
}

struct Repo {
    _dir: tempfile::TempDir,
    path: PathBuf,
    tree: Tree,
    links: Links,
}

impl Repo {
    fn new() -> Repo {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("url-shortener");
        copy(&fixture(), &path);
        git::init_and_commit(&path, "The URL shortener, from its model").unwrap();
        let tree = parse(&[Source::new("UrlShortener.sysml", MODEL)]);
        let links = links(&tree);
        Repo {
            _dir: dir,
            path,
            tree,
            links,
        }
    }

    fn executor(&self) -> Executor {
        Executor::new(Scope::read_only(&self.path).unwrap())
            .trusted(true)
            .target_dir(self.path.parent().unwrap().join("target"))
    }

    fn read(&self) -> impl Fn(&str) -> Option<String> + '_ {
        move |path| std::fs::read_to_string(self.path.join(path)).ok()
    }

    fn edit(&self, file: &str, from: &str, to: &str) {
        let path = self.path.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{from:?} not in {file}");
        std::fs::write(&path, text.replace(from, to)).unwrap();
    }

    fn scenarios(&self) -> Vec<(String, agq_simulation::RunResult)> {
        self.tree
            .walk()
            .into_iter()
            .filter(|id| self.tree[*id].kind == ElementKind::VerificationDef)
            .map(|id| {
                let program = compile(&self.tree, id).unwrap();
                let result = run_implementation(
                    &program,
                    model_digest(&self.tree, id),
                    &Request::new(Mode::Implementation),
                    &self.links,
                    &self.path,
                    &self.executor(),
                    Arc::new(AtomicBool::new(false)),
                );
                (self.tree.effective_name(id).unwrap().to_string(), result)
            })
            .collect()
    }
}

fn passing(results: &[(String, agq_simulation::RunResult)]) -> Vec<&str> {
    results
        .iter()
        .filter(|(_, r)| r.all_passed())
        .map(|(name, _)| name.as_str())
        .collect()
}

#[test]
fn the_url_shortener_keeps_its_model_and_a_break_is_found_and_repaired() {
    let repo = Repo::new();
    // The code keeps the model's boundaries and contracts, and its tests pass.
    let boundaries = module_boundaries(&repo.tree, &repo.links, &repo.read());
    assert_eq!(boundaries.verdict, Verdict::Passed, "{boundaries:#?}");
    let shapes = contract_shapes(&repo.tree, &repo.links, &repo.read());
    assert_eq!(shapes.len(), 13);
    for shape in &shapes {
        assert_eq!(shape.verdict, Verdict::Passed, "{shape:#?}");
    }
    let tests = linked_tests(&repo.links, &repo.executor(), "", Duration::from_secs(600));
    assert_eq!(
        tests.len(),
        4,
        "the service's three and the typed client's conformance"
    );
    for test in &tests {
        assert_eq!(test.verdict, Verdict::Passed, "{test:#?}");
    }
    // The scenarios with stand-ins pass against the real code. The agent's
    // own evaluation cases need a model the harness does not call: the run
    // stops and says so, and nothing claims they passed.
    let results = repo.scenarios();
    assert_eq!(
        passing(&results),
        [
            "ShortenAllowed",
            "ShortenBlocked",
            "ScreeningTimesOut",
            "InvalidScreeningOutput",
            "ReviewRequired",
            "BlocklistedWhenScreeningRefuses"
        ],
        "{:#?}",
        results
            .iter()
            .map(|(n, r)| (n, r.status, &r.stop))
            .collect::<Vec<_>>()
    );
    // The chat agent's cases and the typed agent's (C-52) alike.
    for name in ["ScreeningCases", "TypedScreeningCases"] {
        let (_, cases) = results.iter().find(|(n, _)| n == name).unwrap();
        assert_eq!(cases.status, RunStatus::Stopped, "{name}");
        assert!(
            cases
                .stop
                .as_ref()
                .unwrap()
                .message
                .contains("calls a model"),
            "{name}: {:?}",
            cases.stop
        );
        assert!(cases.checks.iter().all(|c| c.verdict == Verdict::NotRun));
    }

    // The break: a link that needs review goes live.
    repo.edit(
        "src/api.rs",
        "Decision::Review => LinkStatus::Held,",
        "Decision::Review => LinkStatus::Active,",
    );
    let broken = repo.scenarios();
    let failing: Vec<&str> = broken
        .iter()
        .filter(|(name, r)| !name.ends_with("ScreeningCases") && !r.all_passed())
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(
        failing,
        [
            "ScreeningTimesOut",
            "InvalidScreeningOutput",
            "ReviewRequired"
        ],
        "the scenarios that guard review catch it"
    );
    let (_, review) = broken.iter().find(|(n, _)| n == "ReviewRequired").unwrap();
    let held = review.checks.iter().find(|c| c.name == "isHeld").unwrap();
    assert_eq!(held.verdict, Verdict::Failed);
    assert!(held.message.contains("active"), "{}", held.message);
    // The linked test fails too, at the requirement it checks: drift.
    let tests = linked_tests(&repo.links, &repo.executor(), "", Duration::from_secs(600));
    let drifted = drift(&tests);
    let review_requirement = repo
        .tree
        .find("UrlShortener::reviewBeforeActivation")
        .unwrap();
    assert!(drifted.contains_key(&review_requirement), "{drifted:#?}");
    // So does the typed client's conformance to the frozen replies (C-52):
    // its `review` verdicts no longer hold the link.
    let typed = repo.tree.find("UrlShortener::TypedLinkScreening").unwrap();
    assert!(drifted.contains_key(&typed), "{drifted:#?}");
    // Repaired: everything passes again.
    repo.edit(
        "src/api.rs",
        "Decision::Review => LinkStatus::Active,",
        "Decision::Review => LinkStatus::Held,",
    );
    assert_eq!(passing(&repo.scenarios()).len(), 6);

    // A boundary the model does not allow: the api reaching into the store
    // instead of through its port.
    repo.edit(
        "src/api.rs",
        "use crate::ports::{LinkStorePort, ScreeningPort};",
        "use crate::ports::{LinkStorePort, ScreeningPort};\n#[allow(unused_imports)]\nuse crate::store::LinkStore;",
    );
    let boundaries = module_boundaries(&repo.tree, &repo.links, &repo.read());
    assert_eq!(boundaries.verdict, Verdict::Failed);
    assert!(
        boundaries.details[0].contains("uses `crate::store`"),
        "{:?}",
        boundaries.details
    );
    let api = repo.tree.find("UrlShortener::LinkApi").unwrap();
    assert_eq!(boundaries.elements, [api.raw()]);
}

/// The typed client asks what the model says (C-52): its pinned model,
/// question and options are the model's `TypedLinkScreening` and its
/// `Decision` values' documentation, so a change on either side without
/// the other fails here, where both are reviewed together.
#[test]
fn the_typed_client_asks_what_the_model_says() {
    let tree = parse(&[Source::new("UrlShortener.sysml", MODEL)]);
    let code = std::fs::read_to_string(fixture().join("src/jev.rs")).unwrap();
    let one_line = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let doc = |id: agq_language::ElementId| {
        tree[id]
            .children()
            .iter()
            .find(|c| tree[**c].kind == ElementKind::Doc)
            .and_then(|d| tree[*d].text.clone())
            .map(|text| one_line(&text))
            .unwrap()
    };
    let typed = tree.find("UrlShortener::TypedLinkScreening").unwrap();
    assert!(
        code.contains(&format!(
            "pub const INSTRUCTIONS: &str = \"{}\";",
            doc(typed)
        )),
        "the question is TypedLinkScreening's documentation"
    );
    let pinned = agq_language::print(&tree)[0]
        .text
        .lines()
        .skip_while(|l| !l.contains("part def TypedLinkScreening"))
        .find_map(|l| l.trim().strip_prefix(":>> model = \""))
        .map(|rest| rest.trim_end_matches("\";").to_string())
        .unwrap();
    assert!(code.contains(&format!("pub const MODEL: &str = \"{pinned}\";")));
    let decision = tree.find("UrlShortener::Decision").unwrap();
    for value in tree[decision].children() {
        if tree[*value].kind != ElementKind::Enum {
            continue;
        }
        let name = tree.effective_name(*value).unwrap();
        let text = one_line(&code);
        let option = format!("(\"{name}\", \"{}\")", doc(*value));
        let spaced = format!("( \"{name}\", \"{}\", )", doc(*value));
        assert!(
            text.contains(&option) || text.contains(&spaced),
            "the option `{name}` is described as the model documents it"
        );
    }
}
