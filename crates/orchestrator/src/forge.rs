//! The repository's host (C-53, ROADMAP §4.15–§4.16): pushing a cycle's
//! branch, opening its pull request, waiting for the repository's own
//! checks, and merging exactly the reviewed commit, with `git` and GitHub's
//! `gh` as exact commands on Execution's allow-list. What is pushed is one
//! commit holding the reviewed tree on top of the last one pushed (or the
//! base), so an earlier attempt's content (a key, a weakened test) never
//! leaves this computer. Never a force-push, never a push to the default
//! branch, never a bypass of the repository's rules: a merge the host
//! refuses is a blocker, not something to work around. (GitHub's branch
//! protection on the default branch is the backstop for everything an
//! agent's own code could do with the Operator's credentials.)

use agq_execution::process::{Finished, Program};
use agq_execution::{Executor, Scope};
use serde_json::Value;
use std::path::Path;
use std::time::{Duration, Instant};

/// How long the repository's checks may take before the cycle stops.
pub const CHECKS_WITHIN: Duration = Duration::from_secs(60 * 60);

pub(crate) fn run(
    repository: &Path,
    words: &[&str],
    timeout: Duration,
) -> Result<Finished, String> {
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

/// The URL of `repository`'s remote `origin`, if it has one: recorded when
/// an objective is created (C-55), so the approved baseline is read from
/// that remote whatever an agent later does to `origin`.
pub fn origin_url(repository: &Path) -> Option<String> {
    run(
        repository,
        &["git", "remote", "get-url", "origin"],
        Duration::from_secs(60),
    )
    .ok()
    .map(|found| without_credentials(found.stdout.trim()))
    .filter(|url| !url.is_empty())
}

/// `url` without the user and password a URL with a scheme may carry
/// (`https://user:token@host/…`, `https://token@host/…`), so no token is
/// recorded or printed; git asks its credential helper instead. An `ssh://`
/// URL keeps its user (which names no secret) but never a password; an
/// address without a scheme (`git@host:owner/repo`, a path) is kept.
pub fn without_credentials(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, path) = match rest.find('/') {
        Some(at) => rest.split_at(at),
        None => (rest, ""),
    };
    let Some((user, host)) = authority.rsplit_once('@') else {
        return url.to_string();
    };
    let kept = if scheme.eq_ignore_ascii_case("ssh") {
        let name = user.split(':').next().unwrap_or_default();
        format!("{name}@")
    } else {
        String::new()
    };
    format!("{scheme}://{kept}{host}{path}")
}

/// A new commit holding `tree`, on `parent`, with `message`.
pub fn commit_tree(
    repository: &Path,
    tree: &str,
    parent: &str,
    message: &str,
) -> Result<String, String> {
    commit_on(repository, tree, &[parent], message)
}

/// A new commit holding `tree`, on each of `parents`, with `message`: on
/// the last one pushed and on a new base, it is pushed without forcing.
pub fn commit_on(
    repository: &Path,
    tree: &str,
    parents: &[&str],
    message: &str,
) -> Result<String, String> {
    let mut words = vec!["git", "commit-tree", tree];
    for parent in parents {
        words.extend(["-p", parent]);
    }
    words.extend(["-m", message]);
    let made = run(repository, &words, Duration::from_secs(60))?;
    let commit = made.stdout.trim().to_string();
    if commit.len() >= 40 && commit.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(commit)
    } else {
        Err(format!("git commit-tree did not say the commit: {commit}"))
    }
}

/// Pushes `commit` as the remote branch `branch` (never the default
/// branch, never forced: the remote branch must be an ancestor).
pub fn push_commit(
    repository: &Path,
    commit: &str,
    branch: &str,
    base_branch: &str,
) -> Result<(), String> {
    if branch == base_branch
        || branch.is_empty()
        || branch.starts_with('+')
        || branch.starts_with('-')
        || branch.contains(':')
    {
        return Err(format!("{branch} is not a branch a cycle pushes"));
    }
    let refspec = format!("{commit}:refs/heads/{branch}");
    run(
        repository,
        &["git", "push", "origin", &refspec],
        Duration::from_secs(300),
    )
    .map(|_| ())
}

/// Whether `ancestor` is an ancestor of `commit`.
pub fn is_ancestor(repository: &Path, ancestor: &str, commit: &str) -> Result<bool, String> {
    let program = Program::new("git", &["merge-base", "--is-ancestor", ancestor, commit]);
    let executor = Executor::new(Scope::read_only(repository).map_err(|e| e.to_string())?)
        .trusted(true)
        .allow(vec![program.clone()]);
    let finished = executor
        .run(&program, "", Duration::from_secs(60))
        .map_err(|e| format!("{}: {e}", program.display()))?;
    Ok(finished.success)
}

/// Puts a new worktree's branch at `commit` (a cycle starts at its base).
pub fn start_at(worktree: &Path, commit: &str) -> Result<(), String> {
    run(
        worktree,
        &["git", "reset", "--hard", commit],
        Duration::from_secs(120),
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

/// The log of the failed job a failed check links to (`gh run view
/// --log-failed`), from the link in the check's detail
/// (`…/actions/runs/<run>/job/<job>`).
pub fn failed_log(repository: &Path, detail: &str) -> Result<String, String> {
    let (run_id, job) =
        run_and_job(detail).ok_or_else(|| format!("the failed check names no run: {detail}"))?;
    let mut words = vec!["gh", "run", "view", run_id.as_str(), "--log-failed"];
    if let Some(job) = &job {
        words.extend(["--job", job.as_str()]);
    }
    run(repository, &words, Duration::from_secs(180)).map(|f| f.stdout)
}

/// The run and job a check's link names.
pub fn run_and_job(detail: &str) -> Option<(String, Option<String>)> {
    let number_after = |marker: &str| {
        detail
            .split(marker)
            .nth(1)
            .map(|rest| {
                rest.chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>()
            })
            .filter(|n| !n.is_empty())
    };
    Some((number_after("/runs/")?, number_after("/job/")))
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
    let n = number.to_string();
    // Merged already (a restart after the merge): the merge commit, if it
    // was this commit that was merged.
    let state = |tries: u32| -> Result<Value, String> {
        let mut last = String::new();
        for _ in 0..tries {
            match run(
                repository,
                &[
                    "gh",
                    "pr",
                    "view",
                    &n,
                    "--json",
                    "state,mergeCommit,headRefOid",
                ],
                Duration::from_secs(120),
            ) {
                Ok(found) => return serde_json::from_str(&found.stdout).map_err(|e| e.to_string()),
                Err(error) => {
                    last = error;
                    std::thread::sleep(Duration::from_secs(5));
                }
            }
        }
        Err(last)
    };
    let merged_commit = |value: &Value| -> Result<String, String> {
        if value["headRefOid"] != commit {
            return Err(format!(
                "the pull request was merged at {}, not the reviewed commit {commit}",
                value["headRefOid"]
            ));
        }
        value["mergeCommit"]["oid"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| "the host did not say the merge commit".into())
    };
    let before = state(3)?;
    if before["state"] == "MERGED" {
        return merged_commit(&before);
    }
    if before["headRefOid"] != commit {
        return Err(format!(
            "the pull request's head is {}, not the reviewed commit {commit}",
            before["headRefOid"]
        ));
    }
    let merging = run(
        repository,
        &[
            "gh",
            "pr",
            "merge",
            &n,
            "--squash",
            "--delete-branch",
            "--match-head-commit",
            commit,
            "--subject",
            subject,
        ],
        Duration::from_secs(300),
    );
    if let Err(error) = merging {
        // The host may have merged although the command failed (a time
        // out): what it says now decides.
        match state(3) {
            Ok(now) if now["state"] == "MERGED" => {}
            _ => return Err(error),
        }
    }
    let after = state(5)?;
    if after["state"] != "MERGED" {
        return Err(format!(
            "the pull request is {}, not merged",
            after["state"]
        ));
    }
    merged_commit(&after)
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

#[cfg(test)]
mod tests {
    use super::without_credentials;

    /// C-55: the recorded `origin` never holds a token.
    #[test]
    fn a_remotes_credentials_are_never_recorded() {
        for (given, kept) in [
            (
                "https://x-access-token:ghp_0123456789abcdef@github.com/o/r.git",
                "https://github.com/o/r.git",
            ),
            (
                "https://ghp_0123456789abcdef@github.com/o/r.git",
                "https://github.com/o/r.git",
            ),
            (
                "ssh://git:secret@github.com/o/r.git",
                "ssh://git@github.com/o/r.git",
            ),
            (
                "ssh://git@github.com/o/r.git",
                "ssh://git@github.com/o/r.git",
            ),
            ("https://github.com/o/r.git", "https://github.com/o/r.git"),
            ("git@github.com:o/r.git", "git@github.com:o/r.git"),
            ("C:/work/origin.git", "C:/work/origin.git"),
            ("file:///C:/work/origin.git", "file:///C:/work/origin.git"),
        ] {
            assert_eq!(without_credentials(given), kept, "{given}");
        }
    }
}
