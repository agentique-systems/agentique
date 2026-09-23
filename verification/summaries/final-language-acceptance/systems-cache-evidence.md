# Lossless Systems graph cache transport

The historical dependent graph archive omits local selected ordered-reference
contribution proofs/searches. Aggregate graph and certificate digests do not bind
these individual evidence payloads. Systems cache issuance must retain them.

The kernel now provides `write_dependent_overlay_with_evidence` and
`read_dependent_overlay_with_evidence`, using the distinct strict format
`agq-kernel-dependent-evidence-archive/1`. Its dependency pin also includes the
supplied dependency's selected proof/search support, beyond aggregate graph
bytes. Its graph payload preserves canonical positions, proofs, searches,
declared identity reservations, and shared dependency allocation. It contains no
scheduler state and grants no publication authority. Legacy root/dependent and
unaccepted frontier formats remain separate and unchanged.

Systems cache writing, trusted reading, and authentication of the decoded graph
all use the new format. No trusted catalogue, operational default, scheduler,
semantic context, or accepted KerML bytes changed. Receipt authority is still
required before trusted Systems restoration can run.

Synthetic tests compare complete selected contribution structures and rewritten
archive bytes, reject format substitution and malformed selected support, verify
the unchanged frontier payload, retain legacy round trips, and reject an exact
aggregate dependency whose selected evidence was lost. The held accepted Systems
cache test additionally requires nonempty local selected evidence and compares
its exact positions and proof/search content hashes across trusted restoration.
Those hashes retain neither pointer layout nor a second large graph.

## Actual commands and results

Worktree: `agentique-frontier-checkpoint`. All Cargo checks use:

```powershell
$env:CARGO_TARGET_DIR='C:/Users/phili/github/agentique-systems/agentique/target/bridge-self-model'
$env:CARGO_BUILD_JOBS='1'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_INCREMENTAL='0'
```

| Command | Exit | Actual output |
| --- | --- | --- |
| `cargo test --locked --offline -p agq-kernel --test archive` (initial) | 1 | 9 passed, 2 failed: new fixture incorrectly instantiated abstract Relationship. Corrected fixture to concrete Membership; no production semantic change. |
| Same command after fixture correction | 0 | 11 passed, 0 failed, 0 ignored, 0.04 seconds; build 1.89 seconds |
| `cargo test --locked --offline -p agq-kerml-text --test accepted_systems_cache --no-run` | 0 | Accepted cache test executable compiled; build 46.40 seconds. No accepted-cache test executed. |
| `cargo test --locked --offline -p agq-kernel --test archive` (after exact dependency evidence pin) | 0 | 11 passed, 0 failed, 0 ignored, 0.05 seconds; build 8.06 seconds |
| `cargo clippy --locked --offline -p agq-kernel -p agq-kerml-text --all-targets -- -D warnings` | 0 | Finished dev profile in 27.55 seconds, no warnings |
| `cargo fmt --all -- --check` | 0 | No output |
| `git diff --check` | 0 | No output |

The actual accepted Systems cache test remains gated on full publication and
compiled receipt activation. No corpus run or producer replay was performed.
