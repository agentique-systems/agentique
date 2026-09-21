# Verification evidence

Follow [ADR 0021](../docs/adr/0021-verification-evidence-policy.md) for all new work.

- `scripts/`: shared reproducible checks.
- `fixtures/`: regression inputs.
- `summaries/`: concise milestone reports and small curated machine-readable outcomes.
- `authority/`: intentionally retained authority where not already in standards/.
- `generated/`: ignored local reports, logs and temporary evidence.
- Milestone directories: one README, summary, curated inventories and scripts.

Record actual commands, exit codes, source identity and relevant digests in one
milestone summary. Keep full command output and resource observations under
`generated/` or in CI artifacts. A completed command is not semantic acceptance.
New milestones must not copy the historical command-capture directory pattern.

Historical authority packets, offline proof inputs, fixtures and curated summaries
remain in their original directories. Routine logs, repeated per-command source
hashes, nonvisual-test screenshots and superseded audit exports may be removed
from the current tree after checking their consumers. Do not rewrite Git history,
modify normative bytes or remove unique proof inputs to reduce evidence size.

The [current-tree cleanup report](summaries/repository-hygiene.md) identifies the
archive revision for removed raw captures. Historical command indexes, result
wrappers and preservation manifests describe their original revision: their raw
output paths and hashes are historical provenance, not current-tree inputs.
Use the archive revision when reproducing a historical milestone. Retained
independent proof scripts keep the specific input files they require locally.
