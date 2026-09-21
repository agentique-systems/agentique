# Current-tree verification cleanup

This cleanup applies ADR 0021 without rewriting history or changing authority.
The archive revision is
[`ed2cf3a9cc086e165c075c591a3892472250e9b7`](https://github.com/agentique-systems/agentique/tree/ed2cf3a9cc086e165c075c591a3892472250e9b7/verification).
Historical command indexes, result wrappers and preservation inventories refer to
paths at that revision. Retrieve any removed capture with
`git show ed2cf3a9cc086e165c075c591a3892472250e9b7:<repository-relative-path>`.

The inventory found 3,598 tracked files totaling 726,385,720 bytes under
`verification/`. The five largest directories were:

| Directory | Bytes before cleanup |
| --- | ---: |
| `kerml-name-resolution-errata-publication-v4` | 398,144,909 |
| `kerml-library-content-errata-publication-v5` | 79,296,904 |
| `kerml-semantic-closure-v6` | 52,432,737 |
| `kerml-operational-errata-publication-v3` | 44,019,170 |
| `kerml-standard-library-publication-v2` | 42,986,311 |

Removed 916 files totaling 149,337,759 bytes (20.6% of the original bytes):

- 308 repeated `input-hashes.json` and 374 routine `output.txt` captures.
- 206 routine build, format, test, readiness and preservation stdout logs.
- 24 ordinary console screenshots captured during language-only milestones.
- Three superseded v4 obligation exports (`obligations-1`, `obligations-3`,
  `obligations-4`), replaced historically by the retained final profile audits.
- One 8.9 MB failed development Clippy transcript superseded by passing checks.

The original 378 `output.txt` files contained 51 groups of byte-identical logs,
including 114 redundant copies. Four raw outputs and four per-command hash dumps
remain because retained independent proof scripts consume them. All 377 compact
`result.json` and 188 `results.json` records remain: these preserve exact command
arguments and outcomes, and historical scripts use them to reproduce summaries.
Their output fields locate historical captures, not required current-tree files.

The existing tracked evidence population is reduced to 2,682 files before adding
this report. Large final obligation reports, reference oracles and declaration
exports remain where offline verifiers use them. No authority packet, OMG issue
capture, exact pilot/reference patch, operational errata, normative hash, curated
summary, regression fixture, ADR or reproduction script was deleted. Generation-1
release evidence and traceability remain unchanged.

`git grep` inspected raw-capture names and candidate paths before removal. An
additional retained-script path audit found one documentary test-log reference;
it now points to the archive revision. Historical milestone READMEs link here.
The generic verification README explains archive interpretation, and `.gitignore`
blocks new command-capture triplets outside generated storage. New milestones use
`verification/generated/` for raw output and concise summaries for reviewed facts.

The consumer search command exited zero:

```text
git grep -n -E 'input-hashes\.json|output\.txt' -- 'verification/**/*.py' 'verification/**/*.mjs' 'verification/**/*.md'
```

Checks on this worktree:

| Command / check | Exit / result |
| --- | --- |
| `git ls-files verification/` and Python byte/name/object-identity inventory | 0; counts above |
| Consumer search above | 0; proof consumers retained |
| Retained Markdown link and script-input audit | 0; 648 local links inspected; no deleted file is a link target or literal script input |
| `git diff --check` | 0 after preserving original line endings in modified documents |
| Standards, generation-1 release records, and non-candidate evidence preservation audit | 0; unchanged |

The first whitespace check caught Windows newline conversion in edited READMEs;
the files were restored to their original newline convention before the passing
check. No Rust, frontend or corpus command was needed for this evidence-only
workstream. Integrated implementation acceptance is recorded by the lead agent.
