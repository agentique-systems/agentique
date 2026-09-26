# Same-runner exact semantic oracle: correctness passed, performance regressed

[Windows CI run 36231357374](https://github.com/agentique-systems/agentique/actions/runs/36231357374)
built and ran baseline `bd72aee4e1bf2aa2b894dc0201bcef111ddf337b` and current
`a6e1f41d2b549a8be10a79dfa3ec786c090b65b3` on the same runner with the same
portable test harness, accepted runtime and default thin-LTO release settings.
Command and cold reconstruction ran in separate processes. Both passed all
697,419 exact observations and 16 malformed reuse rejections. The observation
maps include full applicable query values, evidence, completeness, diagnostics,
canonical facts, relationship occurrences, derived proof/search support and
closure identities. Only newly allocated kernel revision labels are normalized.

| Boundary / work | Baseline command | Current command | Current cold |
| --- | ---: | ---: | ---: |
| Prepare / restore | 161.497 s | 219.531 s | 163.268 s |
| Compilation | 157.901 s | 215.865 s | 159.964 s |
| Final closure, inclusive | 84.340 s | 142.151 s | 84.554 s |
| Checkpoint capture, nested in closure | not separately measured | 31.030 s | 0.011 ms |
| Checkpoint rebind, nested in closure | not separately measured | 28.556 s | 0 |
| Effective audit | 49.359 s | 50.002 s | 50.631 s |
| Producer subjects evaluated | 717 | 717 | 717 |
| Audit subjects evaluated | 791 | 425 | 791 |
| Audit subjects reused | 0 | 366 | 0 |
| Effective audit checks reused | 0 | 0 | 0 |
| Declared records retained | 0 | 568 | 0 |
| Declared records rebuilt | 1,101 | 533 | 1,101 |
| Documents lowered | 18 | 1 | 18 |

The command is approximately 36% slower than the same-runner baseline and 34%
slower than its independent cold reconstruction. Reduced construction/audit
subject counts do **not** establish a wall-time win. Reused audit subjects had
zero applicable query checks; the expensive checks still ran. The final producer
scheduler did the same work after checkpoint transport because the new declared
frontier did not retain the old local derived outputs.

Checkpoint capture and rebinding hash the unchanged dependency's extensive proof
graph twice. A subsequent separately tested change factors only the exact shared
immutable dependency, retaining every local incoming/proof/search change and
rejecting equal-but-distinct mounts. This artifact predates that change and
cannot qualify its correctness or timing.

`artifact-manifest.json` binds the original downloaded bytes. The four 106 MB
observation JSON files are retained losslessly as `.json.gz`; every original
uncompressed byte count and SHA-256 is recorded. Executables remain in the
original CI artifact, with their exact hashes and build commands retained here.
No source/runtime cache or operator repository is included. Raw observer logs and
sampled process-tree memory remain alongside each oracle invocation. Whole
process times include independent observation export and are not candidate
preparation timings.

## Restore exact evidence into a fresh checkout

Use Python 3.12 or newer from the repository root. The destination must be absent
and Git-ignored under `verification/generated`. This exact destination recreates
the paths in the historical `verification/native-studio-acceptance-proof-set-01.json`:

```powershell
python verification/native-studio-acceptance/restore_ci_oracle.py --retained verification/native-studio-acceptance/ci-oracle-01 --output verification/generated/native-studio-acceptance/ci-oracle-01 --manifest-sha256 8ff048aae7b77ad84b2bb0d88efa46a7a8138959990f85f8a20770f2b7c4a5e8
if ($LASTEXITCODE -ne 0) { throw 'Oracle evidence restoration failed' }
```

The helper validates all 62 retained files before creating output, then rechecks
each stored and restored hash/size while writing. It streams the four large maps,
restores original bytes including line endings, rejects unsafe paths, links,
duplicate/colliding paths and executable entries, and refuses even an existing
empty destination. It never runs the retained scripts. A success receipt is
written last. A write failure may leave an incomplete directory without that
receipt; do not treat such a directory as restored evidence or overwrite it.
Restoration needs about 426 MB. No binaries are restored; obtain the immutable
original CI artifact separately if executable inspection is needed.

Add `--verify-only` to stream-check the complete evidence without writing any
output. In this session that mode passed all 62 files / 426,162,725 original bytes
in 1.234 seconds; the actual command and output are retained in
`../restore-ci-oracle-verification-01.json`. Six small-fixture adversarial tests
passed; their command/output receipt is `../restore-ci-oracle-tests.json`.

If the original download directory already exists, preserve it and choose a
different fresh generated destination. The historical proof set intentionally
keeps its original paths and hashes; a different evidence location requires a
separately reviewed proof-set file, not silent mutation of the historical proof.
Restoring evidence cannot qualify later semantic source edits. Proposal01 remains
historical after the shared-dependency checkpoint optimization, and its source
checks must reject the changed implementation.
