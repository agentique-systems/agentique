# First integration CI platform results

Run: https://github.com/agentique-systems/agentique/actions/runs/36229487866

Exact tested commit: `cf2e811319a96830e52c173051da5e004f6a56d2`. This predates later native state and layout corrections. The recorded job metadata is in `ci-36229487866.json`; the Linux job's complete output is in `ci-linux-native-36229487866.txt`.

| Platform/job | Observed result | Scope |
| --- | --- | --- |
| Linux `native-studio`, job 108369800150 | Clippy exit 101 | Native formatting passed; Clippy failed before tests; native tests were skipped. |
| macOS `native-studio-macos`, job 108369800256 | Build/link success | `cargo build --locked --offline --manifest-path crates/studio-native/Cargo.toml --target-dir target` completed successfully. This is compile/link qualification only. |

The Linux Clippy diagnostic was a single `clippy::collapsible_if` in `src/real_automation.rs:1279`, around the `soak::next_cycle` completion guard. It was assigned to the interaction owner and corrected in `7ada0831` (integrated as `99e6ea1d`). It was not a Linux test failure, and it does not establish that the latest commit passes Linux tests. A new CI run is needed for that claim. Neither job establishes interactive qualification.

Read-only evidence commands:

* `gh run view 36229487866 --repo agentique-systems/agentique --job 108369800150 --log-failed` — exit 1: `run 36229487866 is still in progress; logs will be available when it is complete`.
* `gh api repos/agentique-systems/agentique/actions/jobs/108369800150/logs` — exit 0; full output retained in the Linux text log.
* `gh run view 36229487866 --repo agentique-systems/agentique --json jobs,headSha,status,conclusion` — exit 0; output retained in JSON.

The job evidence was initially fetched in the isolated worlds worktree. That worktree disappeared during concurrent work, so these read-only GitHub requests were repeated to retain their outputs in the integration checkout. No release or workflow was changed by these requests.

## Local UI follow-ups

The integration lead's `acceptance-native-tests-02.txt` reported 145 passed and one failure: the Diff review remained 127 px tall against its unchanged <=110 px requirement. The scoped compact-toolbar fix subsequently passed the targeted test in `acceptance-native-history-03.txt` (1 passed, 145 filtered out). Inspector long-name width passed the earlier full retest.

Commit `dc15cabf` adds the distinct authenticated worker epoch used to enable New Project in an empty repository. Its local format and whitespace checks exited 0 with no output:

* `cargo fmt --manifest-path crates/studio-native/Cargo.toml --all -- --check`
* `git diff --check`

Its no-model reply-routing/rendered-heading test has been sent to the integration lead for the coordinated native retest. It distinguishes a displayed fixture, phase progress, and stale/nonterminal Ready from a successful matching host-open reply. No runtime acceptance claim comes from those test messages.
