# Accepted runtime recovery — native Studio

The 2026-09-25 bounded search did not recover an accepted publication pair.
Native fixture rendering and in-process architecture work continue independently.
No standard was republished, downloaded on startup, or represented as accepted
from a filename. No cache candidate was found to authenticate.

The exact commands, outputs and exit codes are retained in
`runtime-recovery-commands.json`; the read-only search is reproducible with
`python verification/native-studio/runtime-recovery.py`.

## Evidence

- GitHub releases API returned an empty array, exit 0.
- GitHub Actions returned 81 artifacts, all `verification-evidence`. The newest
  is artifact `10849224418`, 145,709,278 bytes. The preceding First Light mission
  downloaded and inventoried the immediately preceding artifact/run
  `36098903068`: 2,950 files and no accepted publication archives. See
  `verification/summaries/agentique-studio-first-light/asset-search.md`.
- The only workflow remains `.github/workflows/ci.yml`; it uploads verification
  and test results. It has no accepted runtime distribution step.
- All-ref Git object lookup for runtime packages and named accepted caches
  returned no objects, exit 0.
- Filename search of available GitHub checkouts, Downloads, Documents, Desktop,
  OneDrive, `.cache`, `.local`, `C:/home` and `C:/Sandbox` returned no matching
  accepted cache or package paths (`rg` exit 1, no errors). This search includes
  hidden and ignored files, excluding regenerable target/node_modules trees.
- Historic worktree entries are prunable because their directories are gone.
  The current checkout and three native mission worktrees are present.
- `C:/Users/phili/.agentique` does not exist.

The search does not establish that every external developer archive is empty.
The new Actions evidence artifact was not downloaded again: no distribution
workflow exists and the adjacent artifact was already inventoried. Artifact
name/size alone is not authentication.

## Exact identification and recovery handoff

The accepted KerML semantic publication digest is
`815573353973607bc62a25195ed4461645182027407f8476be521ffc99174f12`.
Its historical cache length is 540,739,848 bytes. The accepted Systems publication
digest is `25aeddb099be16462b553d6debcad97f43bf22e89ce8a9bbb53cf72eb7d193fa`;
the recorded original cache transport SHA-256 is
`cfa48799aee1abf0e469c07380881b8d9a32c273bd3943713a47473808e8d5f7`.
These identifiers come from the existing accepted receipts and final artifact
gate, not from a new authority declaration. No historical KerML transport hash
was invented.

1. A maintainer with the original publishing machine/storage should restore the
   two original accepted cache files from a retained disk, backup or developer
   archive. Match the Systems transport hash and historical KerML length to
   identify candidates, then authenticate both with the existing facades.
2. Run the existing `agq-publications pack --kerml-cache <path>
   --systems-cache <path> --output <absolute-path>/accepted-runtime.agq-runtime`.
   `pack` authenticates the pair; digest matching alone cannot accept it.
3. Run `agq-publications verify --bundle <absolute-path>/accepted-runtime.agq-runtime`.
   Retain command outputs, exit codes, outer SHA-256 and generated manifest.
4. Install the package through `agq-runtime-publications` and run native First
   Light against `models/agentique`. Real project acceptance remains pending
   until this succeeds. Preserve both publication IDs and immutable source bytes.
5. If original bytes cannot be recovered, prepare a separate publication recovery
   proposal with scope, authority and validation obligations for explicit user
   authorization. This mission grants no permission to republish for convenience.

Release packaging and verification commands are documented in
`docs/runtime-publication-distribution.md`; publication and release distribution
remain separate operations.

## In-process product path

`agq-studio-platform` wraps the existing runtime loader, modeling service,
view projections, agent candidate contracts and durable repository adapter.
The native shell can render setup before the service exists; installed startup
authenticates both publications before creating/opening the model database.
First run shares the existing resumable source-backed Agentique import with the
web host. Existing authored projects and intentionally empty revisions remain
preserved. No HTTP or transport JSON is required for local modeling work.

Missing/rejected runtime tests establish startup failure behavior only. Neither
those tests nor visual fixtures establish real-self-model semantic acceptance.
