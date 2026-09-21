# ADR 0021: verification evidence policy

Status: accepted for publication convergence and subsequent milestones.

## Decision

Git retains durable authority and reproducible verification inputs, not routine
command transcripts. This policy applies repository-wide to new evidence. It
supersedes the append-only, one-directory-per-command convention in v1–v10.
Historical evidence remains historical; no history rewrite or bulk removal is
required.

Keep normative artifact manifests and hashes, reviewed operational errata,
authority blocker registers, concise milestone READMEs, reproducible scripts,
small curated JSON summaries used by tests, regression fixtures, CI configuration,
and intentionally retained external authority bytes whose exact identity matters
and whose retention is permitted. Acquisition remains a maintenance action,
never a build step.

Do not routinely commit per-command input-hashes.json, output.txt or result.json,
successful cargo/npm logs, ordinary regression screenshots, duplicate preservation
snapshots, rerun directories, compiler logs, or repeated facts derivable from Git
and one manifest. Store those in CI artifacts or ignored verification/generated/;
an optional compressed external evidence artifact is also acceptable.

## Layout and reproducibility

verification/README.md defines the layout. Shared scripts live in
verification/scripts/, fixtures in verification/fixtures/, curated summaries in
verification/summaries/, and retained authority in verification/authority/ when
not already under standards/. A milestone may colocate these durable files in
one named directory. Empty directories need not be checked in.

One milestone summary records actual command arguments, exit status, tool/version,
source commit, and relevant content digest. Raw output lives in generated storage.
A command finishing is not a successful verification result. A failed gate stays
failed; a concise, uniquely informative failure excerpt may be retained. Summaries
must distinguish expected negative checks from acceptance gates.

The source commit identifies the base when changes are uncommitted; a digest of
the working patch and untracked implementation inputs identifies the tested tree.
No generated timestamp or successful log becomes semantic authority. CI uploads
raw outputs separately. Reproduction uses pinned local authority, not acquisition.

## Consequences

Reviewers inspect implementation, fixtures, policy and concise outcomes. Original
HTML/PDF/library bytes and historical authority identities remain unchanged.
Generation-1 release evidence remains independent of generation-2 progress.
