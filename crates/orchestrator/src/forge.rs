//! The repository's host (C-53, ROADMAP §4.15–§4.16): pushing a cycle's
//! branch, opening its pull request, waiting for the repository's own
//! checks, and merging exactly the reviewed commit, with `git` and GitHub's
//! `gh` as exact commands on Execution's allow-list. Never a force-push,
//! never a push to the default branch, never a bypass of the repository's
//! rules: a merge the host refuses is a blocker, not something to work
//! around.

use agq_execution::process::{Finished, Program};
use agq_execution::{Executor, Scope};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// How long the repository's checks may take before the cycle stops.
pub const CHECKS_WITHIN: Duration = Duration::from_secs(60 * 60);

fn run(repository: &Path, words: &[&str], timeout: Duration) -> Result<Finished, String> {
    let program = Program::new(words[0], &words[1..]);
    let executor = Executor::new(Scope::read_only(repository).map_err(|e| e.to_string())?)
        .trusted(true)
        .network(true)
        .allow(vec![program.clone()]);
    let finished = executor
        .run(&program, "", timeout)
        .map_err(|e| format!("{}: {e}", program.display()))?;
    if finished.success {
        Ok(finished)
    } else {
        Err(format!(
            "{} failed: {}",
            program.display(),
            agq_execution::process::last_lines(
                &format!("{}\n{}", finished.stdout, finished.stderr),
                12
            )
        ))
    }
}

/// Pushes `branch` (never the default branch, never forced).
pub fn push(repository: &Path, branch: &str, base_branch: &str) -> Result<(), String> {
    if branch == base_branch
        || branch.is_empty()
        || branch.starts_with('+')
        || branch.starts_with('-')
    {
        return Err(format!("{branch} is not a branch a cycle pushes"));
    }
    run(
        repository,
        &["git", "push", "-u", "origin", branch],
        Duration::from_secs(300),
    )
    .map(|_| ())
}

/// The pull request for `branch`: an open one if there is, else a new one.
/// Returns its number and address.
pub fn pull_request(
    repository: &Path,
    branch: &str,
    base_branch: &str,
    title: &str,
    body: &str,
) -> Result<(u64, String), String> {
    if let Ok(found) = run(
        repository,
        &["gh", "pr", "view", branch, "--json", "number,url,state"],
        Duration::from_secs(120),
    ) {
        let value: Value = serde_json::from_str(&found.stdout).unwrap_or_default();
        if value["state"] == "OPEN"
            && let Some(number) = value["number"].as_u64()
        {
            return Ok((
                number,
                value["url"].as_str().unwrap_or_default().to_string(),
            ));
        }
    }
    let created = run(
        repository,
        &[
            "gh",
            "pr",
            "create",
            "--base",
            base_branch,
            "--head",
            branch,
            "--title",
            title,
            "--body",
            body,
        ],
        Duration::from_secs(180),
    )?;
    let url = created
        .stdout
        .lines()
        .rev()
        .find(|l| l.starts_with("https://"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let number = url
        .rsplit('/')
        .next()
        .and_then(|n| n.parse().ok())
        .ok_or_else(|| {
            format!(
                "gh did not say which pull request it opened: {}",
                created.stdout
            )
        })?;
    Ok((number, url))
}

/// The pull request's head commit, as the host has it.
pub fn head(repository: &Path, number: u64) -> Result<String, String> {
    let n = number.to_string();
    let found = run(
        repository,
        &["gh", "pr", "view", &n, "--json", "headRefOid"],
        Duration::from_secs(120),
    )?;
    let value: Value = serde_json::from_str(&found.stdout).map_err(|e| e.to_string())?;
    value["headRefOid"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "the host did not say the pull request's head".into())
}

/// What the repository's checks say about the pull request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checks {
    /// Every check passed (and there is at least one).
    Passed,
    /// A check failed: which, and where to read it.
    Failed(String),
    /// Some are still running, or none has registered yet.
    Pending,
}

/// Reads the pull request's checks once.
pub fn checks(repository: &Path, number: u64) -> Result<Checks, String> {
    let n = number.to_string();
    // `gh pr checks` exits non-zero while checks are pending or failed, so
    // the output is read either way.
    let program = Program::new("gh", &["pr", "checks", &n, "--json", "name,bucket,link"]);
    let executor = Executor::new(Scope::read_only(repository).map_err(|e| e.to_string())?)
        .trusted(true)
        .network(true)
        .allow(vec![program.clone()]);
    let finished = executor
        .run(&program, "", Duration::from_secs(120))
        .map_err(|e| format!("{}: {e}", program.display()))?;
    let value: Value = match serde_json::from_str(&finished.stdout) {
        Ok(value) => value,
        // No checks registered yet: gh says so in words.
        Err(_) if format!("{}{}", finished.stdout, finished.stderr).contains("no checks") => {
            return Ok(Checks::Pending);
        }
        Err(e) => {
            return Err(format!(
                "the checks could not be read: {e}: {}",
                finished.stderr
            ));
        }
    };
    let all = value.as_array().cloned().unwrap_or_default();
    if all.is_empty() {
        return Ok(Checks::Pending);
    }
    if let Some(failed) = all
        .iter()
        .find(|c| matches!(c["bucket"].as_str(), Some("fail") | Some("cancel")))
    {
        return Ok(Checks::Failed(format!(
            "{} {} ({})",
            failed["name"].as_str().unwrap_or("a check"),
            failed["bucket"].as_str().unwrap_or("failed"),
            failed["link"].as_str().unwrap_or("")
        )));
    }
    if all
        .iter()
        .all(|c| matches!(c["bucket"].as_str(), Some("pass") | Some("skipping")))
    {
        Ok(Checks::Passed)
    } else {
        Ok(Checks::Pending)
    }
}

/// Waits for the repository's checks on the pull request, up to `within`;
/// `stop` ends the wait.
pub fn wait_for_checks(
    repository: &Path,
    number: u64,
    within: Duration,
    stop: &dyn Fn() -> bool,
) -> Result<Checks, String> {
    let started = Instant::now();
    loop {
        match checks(repository, number)? {
            Checks::Pending if started.elapsed() < within => {}
            other => return Ok(other),
        }
        for _ in 0..30 {
            if stop() {
                return Err("stopped".into());
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

/// Merges the pull request (squash), only if its head is still `commit`:
/// the reviewed commit is the one merged. Returns the merged commit on the
/// default branch.
pub fn merge(
    repository: &Path,
    number: u64,
    commit: &str,
    subject: &str,
) -> Result<String, String> {
    let now = head(repository, number)?;
    if now != commit {
        return Err(format!(
            "the pull request's head is {now}, not the reviewed commit {commit}"
        ));
    }
    let n = number.to_string();
    run(
        repository,
        &[
            "gh",
            "pr",
            "merge",
            &n,
            "--squash",
            "--match-head-commit",
            commit,
            "--subject",
            subject,
        ],
        Duration::from_secs(300),
    )?;
    let merged = run(
        repository,
        &["gh", "pr", "view", &n, "--json", "mergeCommit,state"],
        Duration::from_secs(120),
    )?;
    let value: Value = serde_json::from_str(&merged.stdout).map_err(|e| e.to_string())?;
    if value["state"] != "MERGED" {
        return Err(format!(
            "the pull request is {}, not merged",
            value["state"]
        ));
    }
    value["mergeCommit"]["oid"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "the host did not say the merge commit".into())
}

/// Brings the merged commit into the repository's default branch here, by
/// fast-forward only, and only if the working copy has nothing of its own:
/// the Operator's uncommitted work is never touched.
pub fn follow(repository: &Path, base_branch: &str, merged: &str) -> Result<(), String> {
    let changed = agq_execution::git::changed_files(repository).map_err(|e| e.to_string())?;
    if !changed.is_empty() {
        return Err(format!(
            "the working copy has uncommitted changes ({}), so the merged commit was not brought in; commit or put them aside, and the objective goes on",
            changed.into_iter().take(5).collect::<Vec<_>>().join(", ")
        ));
    }
    let head = agq_execution::git::head(repository).map_err(|e| e.to_string())?;
    if head.branch.as_deref() != Some(base_branch) {
        return Err(format!(
            "the working copy is on {}, not {base_branch}",
            head.branch.as_deref().unwrap_or("a detached commit")
        ));
    }
    run(
        repository,
        &["git", "fetch", "origin", base_branch],
        Duration::from_secs(300),
    )?;
    run(
        repository,
        &["git", "merge", "--ff-only", merged],
        Duration::from_secs(120),
    )?;
    let now = agq_execution::git::head(repository).map_err(|e| e.to_string())?;
    if now.commit != merged {
        return Err(format!(
            "the default branch is at {}, not {merged}",
            now.commit
        ));
    }
    Ok(())
}
