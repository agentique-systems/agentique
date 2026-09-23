# Authenticated converged frontier; publication remains unaccepted

The sole full attempt stopped at its authorized wall-time cap: watchdog exit
124, `wall_time`, 3,601.109 seconds. It previously recorded strict publication
stage 27 as Complete at 3,449.324 seconds and durably committed invocation 3,
round 28, ContextualBindings. No publication or resume was run for this audit.

The independently retained journal pin is
`735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5`.
Its ZIP, decoded scheduler state and graph bytes authenticate exactly. The
original executable SHA-256 is
`237d2508dfa46f82390d317827deb4be22775e07d7bfab8e49f4a641d0b7d6c6`,
built from commit `7223938437dc8ab7c684961bfb76eaba6c6c92f0`.

The source-session identity was independently recomputed using the source
identity recipe from that build commit: original checked-in manifest, all 21
original Systems source byte strings in deterministic path order, full scope
(`paths=None`), Operational v2, and accepted KerML Operational v9. It matches
`16c342414231f9ffd6669099527e108f2af2069faaab67f54f296ab3d08e846c`.
The strict graph's external dependency pin also equals the separately accepted
KerML graph pin. These are byte/context checks, not a new semantic publication.

Authenticated state observations:

- `converged=true`, round 28, empty pending worklist; 3,921 scheduler status rows
  are Complete.
- 26,532 applicable pairs, 26,532 closed pairs, zero incomplete pairs, and
  452,052 recorded closed requirements; 75,342 certificate subjects and 87
  producer families.
- 6,033 selected ordered-reference contributions; exact proof/search digest
  `4f8fdbdf1ec88490ef29d0c8b774ca756411d7ea2e4e8f71c9d39b682a6e3583`.
- `report.json`, `accepted-publication.json`, `standard-bindings.json`, and
  `canonical.publication.zip` are all absent from the full output directory.

All artifact, model, certificate, executable, original-source implementation,
and watchdog pins are retained in `full-converged-frontier.json`. This evidence
does not assert final mandatory-reference acceptance or grant trusted authority.

## What the checkpoint does and does not establish

In the executable's source, `producer_worklist.rs:1461` captures the converged
frontier before returning `PublicationClosure` at line 1480. The factory in
`crates/kerml-text/src/sysml/publication.rs` still has the following work after
that boundary:

| Source location | Remaining acceptance work |
| --- | --- |
| `publication.rs:308` | Reconstruct the independently required producer registry; attach final context/certificate and check closure coverage and descriptor identity. |
| `publication.rs:349` | Audit final local KerML capabilities and provenance, then collect final authority conflicts. |
| `publication.rs:371` | Audit all 1,327 mandatory references against the final strict graph. |
| `publication.rs:372` | Revalidate final Systems binding targets and verified declaration sources. |
| `publication.rs:412` | Audit final effective SysML populations. |
| `publication.rs:415` | Reject any accumulated finding before constructing the accepted facade at line 427. |
| `examples/sysml_systems_publication.rs:580` | After factory success, write cache, receipt and binding manifest, run binding stale validation, and write the final report at line 603. |

Input identity, source-document, kernel-obligation, declared source-provenance,
and preliminary producer-binding guards occur *before* the strict scheduler
(`publication.rs:200–254`). Reaching this authenticated strict invocation implies
those earlier guards did not reject this execution; that is a source-path
inference, not a final audit receipt. It does not establish acceptance of final
derived provenance, final reference answers or effective capability queries.

No per-audit completion event or stack trace was recorded after the durable
checkpoint. The exact interrupted post-checkpoint function cannot be identified
from the retained log. The artifact absence and watchdog exit establish that
publication evidence was not issued; scheduler Complete cannot fill that gap.

## Actual commands and exits

Executed in `agentique-frontier-checkpoint`, using Python streaming helpers only:

```powershell
python verification/scripts/audit_stopped_systems_frontier.py --repository C:/Users/phili/github/agentique-systems/agentique --journal C:/Users/phili/github/agentique-systems/agentique/verification/generated/final-language-acceptance/full-frontiers/journal-735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5.json --journal-sha256 735481e45453e104dd2bfe5d1e35bdb4ca89974d71b78dce5c51d267adfb6df5 --output verification/summaries/final-language-acceptance/full-converged-frontier.json
```

Initial exit: **0**, audit 28.125 seconds. Added explicit strict-format and
accepted-KerML graph-pin checks, and compacted the evidence before one final
verification; its actual duration is recorded in the JSON ledger. No Rust build,
producer replay, source-to-model construction, publication or resume occurred.

Final exit: **0**, audit **30.593 seconds**, including strict graph format and
accepted KerML dependency graph pin validation. `git diff --check` also exited 0.
