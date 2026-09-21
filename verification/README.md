# Verification evidence

Follow [ADR 0021](../docs/adr/0021-verification-evidence-policy.md) for all new work.

- `scripts/`: shared reproducible checks.
- `fixtures/`: regression inputs.
- `summaries/`: small curated machine-readable outcomes.
- `authority/`: intentionally retained authority where not already in standards/.
- `generated/`: ignored local reports, logs and temporary evidence.
- Milestone directories: one README, summary, curated inventories and scripts.

Historical milestone directories retain their original evidence conventions.
New milestones must not copy their command-capture directory pattern. Record
actual commands and exit codes in one summary; keep raw output outside Git.
