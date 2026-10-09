//! The gates a cycle's change passes before it is merged (C-53, ROADMAP
//! §4.16), besides the required checks and the review: §4.15's integration
//! checks on every path the change touches, the baseline guard, no
//! configured key in it, and since C-54 the defect shown on the base by at
//! least one criterion with evidence while none passes there
//! ([`defect_shown`]), and user-facing code ([`user_facing`]) evaluated with
//! a behavioural criterion; since C-55, purpose and governance unchanged
//! ([`purpose`]), whatever the objective names. Each ends with an explicit
//! outcome.

use crate::record::{Outcome, Proposal, Review};
use agq_execution::git::{FileChange, Patch};

/// Paths no cycle writes (the task rule, §4.15), besides the links' own.
pub const ALWAYS_PROTECTED: [&str; 6] = agq_implementation::task::ALWAYS_PROTECTED;

/// The agent configuration a cycle changes only when the objective names it.
pub const AGENT_CONFIGURATION: [&str; 4] = agq_assistant::policy::AGENT_CONFIGURATION;

/// Files that configure agents wherever they are (Claude Code reads them in
/// the folder it works in).
const AGENT_FILES: [&str; 5] = [
    "CLAUDE.md",
    "CLAUDE.local.md",
    "AGENTS.md",
    ".mcp.json",
    ".claude",
];

/// Agentique's safeguards: the code that gates, permits and records what
/// agents do. A cycle changes them only when the objective names them, as
/// it names agent configuration; the code of locked parts is gated by the
/// links besides.
pub const SAFEGUARDS: [&str; 8] = [
    "crates/orchestrator",
    "crates/assistant/src/policy.rs",
    "crates/assistant/src/model_tools.rs",
    "claude-agent/src/policy.ts",
    "crates/studio-native/src/control",
    "crates/studio-native/src/objectives.rs",
    "crates/studio-native/src/panels/objectives.rs",
    "crates/implementation/src/task.rs",
];

fn outcome(name: &str, problems: Vec<String>) -> Outcome {
    if problems.is_empty() {
        Outcome {
            name: name.into(),
            verdict: "passed".into(),
            detail: String::new(),
            judged: false,
        }
    } else {
        Outcome {
            name: name.into(),
            verdict: "failed".into(),
            detail: problems.join("\n"),
            judged: false,
        }
    }
}

/// The part whose code is user-facing (C-54): a change to it is evaluated
/// in a test instance whatever its other criteria.
pub const USER_FACING: &str = "AgentiqueArchitecture::Studio";

/// Whether `path` is a test's (one rule for the baseline guard, the files
/// brought over to the base and user-facing code): a file in a directory
/// named `tests`, `test` or `benches` (anywhere, `src/test/` too: such a
/// folder holds tests in Node and Python projects as in this one's
/// `claude-agent/test/`), a Node test file (`.test.` in its name), a Python
/// one (`test_…`), or a Rust file of tests by its whole name (`tests.rs`,
/// `…_tests.rs`, `…_test.rs`; never `contests.rs`).
pub fn test_path(path: &str) -> bool {
    let lower = path.replace('\\', "/").to_lowercase();
    let mut parts: Vec<&str> = lower.split('/').collect();
    let name = parts.pop().unwrap_or_default();
    parts
        .iter()
        .any(|part| matches!(*part, "tests" | "test" | "benches"))
        || name.contains(".test.")
        || name.starts_with("test_")
        || name == "tests.rs"
        || name.ends_with("_tests.rs")
        || name.ends_with("_test.rs")
}

/// The files of `changed` that are user-facing code: the code of the part
/// [`USER_FACING`] by its crate links in any of `links` (the texts of
/// `model/links.json`: the base's and the change's, so dropping a link does
/// not hide the code), outside tests.
pub fn user_facing(changed: &[String], links: &[String]) -> Vec<String> {
    let roots: Vec<String> = links
        .iter()
        .flat_map(|text| {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
            parsed["links"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|l| l["name"] == USER_FACING && l["kind"] == "crate")
                .filter_map(|l| l["path"].as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .collect();
    changed
        .iter()
        .filter(|f| roots.iter().any(|r| under(f, r)) && !test_path(f))
        .cloned()
        .collect()
}

/// The criteria's outcomes on the base show the defect (C-54, requirement
/// `DefectShownBefore`): none passed there, and at least one failed there
/// with evidence (a replay or observation that failed on the base build, or
/// a test run that compiled, ran a test and failed an assertion). An
/// outcome of `no evidence` (no test ran, it did not compile, it timed out,
/// it was not run, a setup failed, a judgment) shows nothing, and is named
/// with its reason. Only the outcomes of `criteria` (the frozen criteria's
/// ids, and the replay's) count.
pub fn defect_shown(before: &[Outcome], criteria: &[String]) -> Outcome {
    let name = "the criteria show the defect on the base";
    let before: Vec<Outcome> = before
        .iter()
        .filter(|o| criteria.contains(&o.name))
        .cloned()
        .collect();
    let before = before.as_slice();
    let passed: Vec<&str> = before
        .iter()
        .filter(|o| o.passed())
        .map(|o| o.name.as_str())
        .collect();
    let mut problems = Vec::new();
    if !passed.is_empty() {
        problems.push(format!(
            "{} already pass on the base, so they do not show the change",
            passed.join(", ")
        ));
    }
    if !before.iter().any(|o| o.verdict == "failed") {
        let why: Vec<String> = before
            .iter()
            .filter(|o| !o.passed())
            .map(|o| format!("{}: {}", o.name, o.detail))
            .collect();
        problems.push(format!(
            "no criterion fails on the base with evidence{}",
            if why.is_empty() {
                String::new()
            } else {
                format!(" ({})", why.join("; "))
            }
        ));
    }
    outcome(name, problems)
}

/// Whether `path` is `pattern` or inside it (a plain path prefix, as the
/// links write them; case does not matter, as on Windows).
fn under(path: &str, pattern: &str) -> bool {
    let path = path.to_lowercase();
    let pattern = pattern.trim_end_matches('/').to_lowercase();
    path == pattern || path.starts_with(&format!("{pattern}/"))
}

/// §4.15 on the paths: protected paths (the links' and the task rule's),
/// the model folder's own rules (only its documents and identities change,
/// and those only through Agentique's tools, which write them), and the
/// agent configuration unless the objective names it.
pub fn paths(patch: &Patch, protected: &[String], configuration: &[String]) -> Outcome {
    let mut problems = Vec::new();
    for file in &patch.files {
        let path = file.path.replace('\\', "/");
        if let Some(rule) = protected
            .iter()
            .map(String::as_str)
            .chain(ALWAYS_PROTECTED)
            .find(|p| under(&path, p))
        {
            problems.push(format!("{path} is protected ({rule})"));
        }
        if under(&path, "model") {
            let name = path.trim_start_matches("model/");
            let document = name.ends_with(".sysml") && !name.contains('/');
            if !(document || name == "agentique.json") {
                problems.push(format!(
                    "{path}: in the model folder only the documents and identities change"
                ));
            }
        }
        let allowed = configuration.iter().any(|n| under(&path, n));
        let agent_file = path
            .split('/')
            .find(|part| AGENT_FILES.iter().any(|f| part.eq_ignore_ascii_case(f)));
        if let Some(config) = AGENT_CONFIGURATION
            .iter()
            .copied()
            .find(|c| under(&path, c))
            .or(agent_file)
            && !allowed
        {
            problems.push(format!(
                "{path} is agent configuration ({config}); the objective does not name it"
            ));
        }
        if let Some(safeguard) = SAFEGUARDS.iter().find(|s| under(&path, s))
            && !allowed
        {
            problems.push(format!(
                "{path} is one of Agentique's safeguards ({safeguard}); the objective does not name it"
            ));
        }
    }
    outcome("paths the change touches", problems)
}

/// The Operator's statement of Agentique's purpose and of how it is
/// governed (C-55): no cycle changes it.
pub const GOVERNANCE: &str = "ROADMAP.md";

/// Purpose and governance stay the Operator's (C-55): a change to
/// [`GOVERNANCE`], or to the model's purpose requirement (`purpose`: what it
/// changes of it, found by identity, `traceability::purpose_changes`; why it
/// could not be checked), fails, even when the objective names it, as no
/// objective unlocks the stable core. A proposal may describe a desired
/// change to the purpose for the Operator; a cycle cannot make it. The
/// agent configuration keeps its own rule ([`paths`]).
pub fn purpose(patch: &Patch, changed: &Result<Vec<String>, String>) -> Outcome {
    let name = "purpose and governance unchanged";
    let mut problems = Vec::new();
    for file in &patch.files {
        let path = file.path.replace('\\', "/");
        if path.eq_ignore_ascii_case(GOVERNANCE) {
            problems.push(format!(
                "{path} states the purpose and the governance, which are the Operator's (C-55): no cycle changes it, even when the objective names it"
            ));
        }
    }
    match changed {
        Ok(changed) => {
            for change in changed {
                problems.push(format!(
                    "the model's purpose requirement changes ({change}): it is the Operator's (C-55), and no cycle changes it, even when the objective names it"
                ));
            }
        }
        Err(error) if problems.is_empty() => {
            return Outcome::new(
                name,
                "not run",
                format!("the purpose requirement could not be checked: {error}"),
            );
        }
        Err(error) => problems.push(format!(
            "the purpose requirement could not be checked: {error}"
        )),
    }
    outcome(name, problems)
}

/// No configured key appears in what the change adds.
pub fn keys(patch: &Patch, keys: &[String]) -> Outcome {
    let mut problems = Vec::new();
    for file in &patch.files {
        let added = file
            .diff
            .lines()
            .filter(|l| l.starts_with('+') && !l.starts_with("+++"));
        for line in added {
            if keys
                .iter()
                .any(|k| k.len() >= 12 && line.contains(k.as_str()))
            {
                problems.push(format!("{} adds a configured key", file.path));
                break;
            }
        }
    }
    outcome("no key in the change", problems)
}

/// No configured key appears in a text that leaves this computer with the
/// change (a commit message, a pull request's title and body).
pub fn keys_in(what: &str, text: &str, keys: &[String]) -> Outcome {
    let problems = if keys
        .iter()
        .any(|k| k.len() >= 12 && text.contains(k.as_str()))
    {
        vec![format!("{what} holds a configured key")]
    } else {
        Vec::new()
    };
    outcome(&format!("no key in {what}"), problems)
}

/// Whether a line of a diff holds what tests and budgets rest on: an
/// assertion, a test, a comparison with a number, or a constant.
fn guarded(line: &str, script: bool) -> bool {
    let line = line.trim();
    // A comparison with a number (a threshold), not an arrow or a generic.
    let compares = [" < ", " > ", " <= ", " >= "]
        .iter()
        .any(|c| line.contains(c))
        && line.chars().any(|c| c.is_ascii_digit())
        && !line.starts_with("//");
    line.contains("assert")
        || line.contains("#[test]")
        || (script && line.contains("expect("))
        || compares
        || (line.starts_with("const ") && line.chars().any(|c| c.is_ascii_digit()))
}

/// What a change does to tests, checks and budgets: the baseline guard's
/// findings for one file.
fn test_changes(file: &FileChange) -> Vec<String> {
    let path = file.path.replace('\\', "/");
    let lower = path.to_lowercase();
    let removed: Vec<&str> = file
        .diff
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .collect();
    let added: Vec<&str> = file
        .diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .collect();
    let count = |lines: &[&str], what: &str| lines.iter().filter(|l| l.contains(what)).count();
    let mut found = Vec::new();
    if file.status == "deleted"
        && (lower.contains("/tests/") || lower.ends_with("_test.rs") || lower.contains(".test."))
    {
        found.push(format!("{path}: a test file is deleted"));
    }
    let tests_removed = count(&removed, "#[test]") + count(&removed, "test(\"");
    let tests_added = count(&added, "#[test]") + count(&added, "test(\"");
    if tests_removed > tests_added {
        found.push(format!(
            "{path}: {} test(s) fewer",
            tests_removed - tests_added
        ));
    }
    if count(&added, "#[ignore") > count(&removed, "#[ignore")
        || count(&added, "test.skip") > 0
        || count(&added, ".only(") > 0
    {
        found.push(format!("{path}: a test is ignored or skipped"));
    }
    // Each assertion, test, threshold or constant removed or changed, line
    // by line (a line moved unchanged is not a change).
    let kept: Vec<String> = added.iter().map(|l| l[1..].trim().to_string()).collect();
    for line in &removed {
        let text = line[1..].trim();
        let script = lower.ends_with(".ts") || lower.ends_with(".js") || lower.ends_with(".mjs");
        if guarded(text, script)
            && !kept.iter().any(|k| k == text)
            && (test_path(&lower) || text.contains("assert"))
        {
            found.push(format!(
                "{path}: removes or changes `{}`",
                text.chars().take(120).collect::<String>()
            ));
        }
    }
    let gating = [
        "#[cfg(any())]",
        "#[cfg(all(any()))]",
        "cfg(FALSE)",
        "#[cfg(not(test))]",
        "#[cfg(never)]",
    ];
    if added.iter().any(|l| gating.iter().any(|g| l.contains(g))) {
        found.push(format!("{path}: code is compiled out"));
    }
    let checks = [
        "tools/check_architecture.py",
        ".github/workflows",
        "budget",
        "performance",
        "stress",
        "crates/implementation/src/task.rs",
    ];
    if checks.iter().any(|c| lower.contains(c)) && file.added + file.removed > 0 {
        found.push(format!(
            "{path}: the checks, budgets or required checks change"
        ));
    }
    found
}

/// The baseline guard: deleted or ignored tests, fewer assertions, and
/// changed checks, budgets or required checks are listed; each fails the
/// cycle unless the proposal named that file as an intended change and the
/// reviewer accepted it.
pub fn baseline(patch: &Patch, proposal: &Proposal, review: Option<&Review>) -> Outcome {
    let accepted = review.is_some_and(|r| r.test_changes_accepted);
    let mut problems = Vec::new();
    for file in &patch.files {
        for finding in test_changes(file) {
            let intended = proposal
                .intended_test_changes
                .iter()
                .any(|c| under(&file.path.replace('\\', "/"), &c.path.replace('\\', "/")));
            if !(intended && accepted) {
                problems.push(if intended {
                    format!("{finding} (intended, but the reviewer did not accept it)")
                } else {
                    format!("{finding} (not named in the proposal)")
                });
            }
        }
    }
    outcome("baseline tests and checks kept", problems)
}

/// The changes to tests and checks, for the reviewer to judge.
pub fn listed_test_changes(patch: &Patch) -> Vec<String> {
    patch.files.iter().flat_map(test_changes).collect()
}

#[cfg(test)]
mod evidence_tests {
    use super::*;

    fn on_base(name: &str, verdict: &str, detail: &str) -> Outcome {
        Outcome::new(name, verdict, detail)
    }

    /// `DefectShownBefore`: one criterion failing on the base with evidence
    /// is needed, and none may pass there; no evidence shows nothing.
    #[test]
    fn the_defect_is_shown_on_the_base_only_with_evidence() {
        let ids = |names: &[&str]| names.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        let all = ids(&["replay", "c1", "c2", "c3"]);
        let defect_shown = |before: &[Outcome]| super::defect_shown(before, &all);
        let shown = defect_shown(&[
            on_base("replay", "failed", "the replay failed on the base build"),
            on_base("c1", "no evidence", "it did not compile"),
        ]);
        assert!(shown.passed(), "{}", shown.detail);
        let none = defect_shown(&[
            on_base("c1", "no evidence", "it ran no test"),
            on_base("c2", "no evidence", "it did not compile"),
            on_base("c3", "not run", "a judgment is no evidence"),
        ]);
        assert!(!none.passed());
        assert!(
            none.detail.contains("c1: it ran no test") && none.detail.contains("did not compile"),
            "{}",
            none.detail
        );
        let passing = defect_shown(&[
            on_base("c1", "failed", "1 test failed"),
            on_base("c2", "passed", ""),
        ]);
        assert!(!passing.passed());
        assert!(passing.detail.contains("c2 already pass"));
        assert!(!defect_shown(&[]).passed(), "no criterion, no evidence");
        // Only the frozen criteria and the replay count: a made-up outcome
        // (a dialog open at the start) is no evidence.
        let made_up = super::defect_shown(
            &[
                on_base("no dialog when it starts", "failed", "a dialog was open"),
                on_base("c1", "no evidence", "setup action failed"),
            ],
            &ids(&["c1"]),
        );
        assert!(!made_up.passed(), "{}", made_up.detail);
        let replay_only = super::defect_shown(
            &[on_base("replay", "failed", "it fails on the base")],
            &ids(&["c1"]),
        );
        assert!(!replay_only.passed(), "no replay was frozen");
    }

    #[test]
    fn user_facing_code_is_the_studios_by_its_links_outside_tests() {
        let links = r#"{"format":1,"links":[
            {"element":932,"name":"AgentiqueArchitecture::Studio","kind":"crate","path":"crates/studio-native"},
            {"element":932,"name":"AgentiqueArchitecture::Studio","kind":"crate","path":"crates/studio-scene"},
            {"element":1379,"name":"AgentiqueArchitecture::Orchestrator","kind":"crate","path":"crates/orchestrator"}
        ]}"#;
        let changed: Vec<String> = [
            "crates/studio-native/src/panels/library.rs",
            "crates/studio-native/tests/journeys.rs",
            "crates/studio-scene/src/lib.rs",
            "crates/orchestrator/src/run.rs",
            "crates/studio-native-extra/src/x.rs",
            "claude-agent/test/policy.test.ts",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            user_facing(&changed, &[links.to_string()]),
            vec![
                "crates/studio-native/src/panels/library.rs".to_string(),
                "crates/studio-scene/src/lib.rs".to_string()
            ]
        );
        assert!(user_facing(&changed, &["not json".to_string()]).is_empty());
        // A change that drops the Studio's links is judged by the base's too.
        let dropped = r#"{"format":1,"links":[]}"#.to_string();
        assert_eq!(
            user_facing(&changed, &[dropped, links.to_string()]).len(),
            2
        );
        assert!(test_path("crates/x/tests/a.rs") && test_path("tools/test_check.py"));
        assert!(!test_path("crates/x/src/testing.rs"));
        assert!(
            test_path("claude-agent/test/policy.test.ts") && test_path("crates/x/benches/b.rs")
        );
        // Whole names: a file whose name ends like one is not one.
        assert!(test_path("crates/x/src/tests.rs") && test_path("crates/x/src/thread_tests.rs"));
        assert!(test_path("crates/x/src/gate_test.rs") && test_path("crates/x/src/test/a.rs"));
        assert!(!test_path("crates/x/src/contests.rs") && !test_path("crates/x/src/latest.rs"));
        assert!(!test_path("crates/x/src/attests.rs") && !test_path("crates/testing/src/lib.rs"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::TestChange;

    fn change(path: &str, status: &str, diff: &str) -> FileChange {
        FileChange {
            path: path.into(),
            status: status.into(),
            added: diff.lines().filter(|l| l.starts_with('+')).count(),
            removed: diff.lines().filter(|l| l.starts_with('-')).count(),
            diff: diff.into(),
        }
    }

    fn proposal(intended: &[&str]) -> Proposal {
        Proposal {
            title: "x".into(),
            kind: "correctness".into(),
            why: "y".into(),
            parts: vec![],
            plan: vec![],
            criteria: vec![],
            intended_test_changes: intended
                .iter()
                .map(|p| TestChange {
                    path: p.to_string(),
                    why: "z".into(),
                })
                .collect(),
            ..Proposal::default()
        }
    }

    /// C-55: ROADMAP.md fails the gate even when the objective names it
    /// (the configuration it may change does not count here); a change to
    /// the purpose requirement fails it too; one that could not be checked
    /// is not run, never a pass; AGENTS.md keeps its rule, allowed only
    /// when named.
    #[test]
    fn purpose_and_governance_fail_even_when_the_objective_names_them() {
        let roadmap = Patch {
            files: vec![change("ROADMAP.md", "modified", "-old\n+new")],
        };
        let named = ["ROADMAP.md".to_string()];
        // Named by the objective, the paths gate lets it through; the
        // purpose gate does not read what the objective names at all.
        assert!(paths(&roadmap, &[], &named).passed());
        let failed = purpose(&roadmap, &Ok(Vec::new()));
        assert!(!failed.passed());
        assert!(failed.detail.contains("ROADMAP.md"), "{}", failed.detail);
        let deleted = Patch {
            files: vec![change("roadmap.md", "deleted", "-old")],
        };
        assert!(!purpose(&deleted, &Ok(Vec::new())).passed());
        let elsewhere = Patch {
            files: vec![change("docs/ROADMAP.md", "added", "+notes")],
        };
        assert!(purpose(&elsewhere, &Ok(Vec::new())).passed());
        let ordinary = Patch {
            files: vec![change("crates/a/src/lib.rs", "modified", "+x")],
        };
        let changed = Ok(vec!["Shop::Purpose::(doc) (doc, updated)".to_string()]);
        let failed = purpose(&ordinary, &changed);
        assert!(!failed.passed());
        assert!(
            failed.detail.contains("Shop::Purpose::(doc)"),
            "{}",
            failed.detail
        );
        let unknown = purpose(&ordinary, &Err("the model cannot be read".into()));
        assert_eq!(unknown.verdict, "not run");
        assert!(!unknown.passed());
        assert!(purpose(&ordinary, &Ok(Vec::new())).passed());
        // AGENTS.md: allowed only when named, as before.
        let agents = Patch {
            files: vec![change("AGENTS.md", "modified", "-a\n+b")],
        };
        assert!(!paths(&agents, &[], &[]).passed());
        assert!(paths(&agents, &[], &["AGENTS.md".into()]).passed());
        assert!(purpose(&agents, &Ok(Vec::new())).passed());
    }

    #[test]
    fn the_paths_a_change_may_not_touch_are_refused() {
        let patch = Patch {
            files: vec![
                change("crates/a/src/lib.rs", "modified", "+x"),
                change("model/Agentique.sysml", "modified", "+x"),
                change("model/links.json", "modified", "+x"),
                change("standards/x.kerml", "modified", "+x"),
                change("CLAUDE.md", "modified", "+x"),
                change(".env", "added", "+KEY=1"),
            ],
        };
        let outcome = paths(&patch, &["standards".into()], &[]);
        assert!(!outcome.passed());
        for path in ["model/links.json", "standards/x.kerml", "CLAUDE.md", ".env"] {
            assert!(outcome.detail.contains(path), "{path}: {}", outcome.detail);
        }
        assert!(!outcome.detail.contains("lib.rs"));
        assert!(!outcome.detail.contains("Agentique.sysml"));
        // Named agent configuration may change.
        let named = paths(
            &Patch {
                files: vec![change("CLAUDE.md", "modified", "+x")],
            },
            &[],
            &["CLAUDE.md".into()],
        );
        assert!(named.passed(), "{}", named.detail);
    }

    #[test]
    fn weakening_a_test_or_a_check_fails_unless_intended_and_accepted() {
        let weakened = Patch {
            files: vec![
                change(
                    "crates/a/tests/a.rs",
                    "modified",
                    "-#[test]\n-fn checks_x() {\n-    assert_eq!(x(), 1);\n-}\n",
                ),
                change("crates/b/src/lib.rs", "modified", "+#[ignore]\n #[test]\n"),
                change(
                    "crates/studio-native/src/budgets.rs",
                    "modified",
                    "-pub const START: u64 = 400;\n+pub const START: u64 = 900;\n",
                ),
            ],
        };
        let failed = baseline(&weakened, &proposal(&[]), None);
        assert!(!failed.passed());
        assert!(failed.detail.contains("1 test(s) fewer"));
        assert!(failed.detail.contains("ignored"));
        assert!(failed.detail.contains("budgets.rs"));
        let review = Review {
            verdict: "approve".into(),
            findings: vec![],
            test_changes_accepted: true,
            commit: "c".into(),
            traceability: String::new(),
            purpose: String::new(),
        };
        let all = [
            "crates/a/tests/a.rs",
            "crates/b/src/lib.rs",
            "crates/studio-native/src/budgets.rs",
        ];
        assert!(baseline(&weakened, &proposal(&all), Some(&review)).passed());
        // Intended but not accepted still fails.
        let rejected = Review {
            test_changes_accepted: false,
            ..review
        };
        assert!(!baseline(&weakened, &proposal(&all), Some(&rejected)).passed());
        // Adding tests is never a finding.
        let added = Patch {
            files: vec![change(
                "crates/a/tests/a.rs",
                "modified",
                "+#[test]\n+fn more() { assert!(true); }\n",
            )],
        };
        assert!(baseline(&added, &proposal(&[]), None).passed());
    }

    #[test]
    fn a_configured_key_in_the_change_is_refused() {
        let patch = Patch {
            files: vec![change(
                "notes.md",
                "modified",
                "+the key is sk-abcdef0123456789\n",
            )],
        };
        assert!(!keys(&patch, &["sk-abcdef0123456789".into()]).passed());
        assert!(keys(&patch, &["sk-other-0123456789".into()]).passed());
    }

    #[test]
    fn agent_configuration_anywhere_and_the_safeguards_need_the_objective() {
        let patch = Patch {
            files: vec![
                change("crates/x/CLAUDE.md", "added", "+be bold"),
                change("crates/orchestrator/src/gates.rs", "modified", "-a\n+b"),
                change("crates/x/src/lib.rs", "modified", "-a\n+b"),
            ],
        };
        let found = paths(&patch, &[], &[]);
        assert!(
            found
                .detail
                .contains("crates/x/CLAUDE.md is agent configuration"),
            "{}",
            found.detail
        );
        assert!(found.detail.contains("safeguards"), "{}", found.detail);
        assert!(
            !found.detail.contains("crates/x/src/lib.rs"),
            "{}",
            found.detail
        );
        let named = paths(
            &patch,
            &[],
            &["crates/x/CLAUDE.md".into(), "crates/orchestrator".into()],
        );
        assert!(named.passed(), "{}", named.detail);
        assert!(
            !keys_in(
                "the body",
                "uses sk-live-0123456789abcdef",
                &["sk-live-0123456789abcdef".into()]
            )
            .passed()
        );
        assert!(
            keys_in(
                "the body",
                "nothing secret",
                &["sk-live-0123456789abcdef".into()]
            )
            .passed()
        );
    }

    #[test]
    fn the_baseline_guard_sees_each_weakened_assertion_threshold_and_gate() {
        let swapped = change(
            "crates/x/tests/a.rs",
            "modified",
            "-    assert_eq!(parse(\"1\"), 1);\n+#[test]\n+fn b() { assert!(true); }",
        );
        let relaxed = change(
            "crates/system-state/tests/performance.rs",
            "modified",
            "-    assert!(ms < 10.0);\n+    assert!(ms < 1000.0);",
        );
        let gated = change(
            "crates/x/src/lib.rs",
            "modified",
            "+#[cfg(any())]\n #[test]",
        );
        let ordinary = change(
            "crates/x/tests/c.rs",
            "modified",
            "-    let project = Project::open(dir).expect(\"opens\");
-fn bytes() -> Vec<u8> { vec![1] }
+    let project = Project::open(&dir).expect(\"opens\");",
        );
        assert!(
            test_changes(&ordinary).is_empty(),
            "{:?}",
            test_changes(&ordinary)
        );
        let moved = change(
            "crates/x/tests/b.rs",
            "modified",
            "-    assert!(ok);\n+    assert!(ok);",
        );
        let found: Vec<String> = [swapped, relaxed, gated, moved]
            .iter()
            .flat_map(test_changes)
            .collect();
        assert!(
            found
                .iter()
                .any(|f| f.contains("a.rs: removes or changes `assert_eq!(parse")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|f| f.contains("performance.rs: removes or changes `assert!(ms < 10.0)")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|f| f.contains("compiled out")),
            "{found:?}"
        );
        assert!(
            !found.iter().any(|f| f.contains("tests/b.rs")),
            "a moved line is no change: {found:?}"
        );
    }
}
